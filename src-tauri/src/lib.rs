use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod icon;
use icon::extract_icon_png;

/// 单个可启动项（快捷方式 / 可执行文件 / 链接 / 脚本）
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LaunchItem {
    /// 显示名称（去掉扩展名）
    pub name: String,
    /// 完整路径
    pub path: String,
    /// 文件扩展名（小写，不含点）
    pub ext: String,
    /// 类型：app / url / script / folder
    pub kind: String,
    /// 是否为“干扰项”（卸载/帮助/网站等，默认折叠）
    pub noise: bool,
    /// 图标（base64 data URL，如 "data:image/png;base64,..."。散装项由 scan_all 统一提取填充）
    #[serde(default)]
    pub icon: String,
}

/// 一个软件（二级目录），含其下的若干快捷方式
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct App {
    /// 软件名（目录名）
    pub name: String,
    /// 软件目录完整路径
    pub path: String,
    /// 该软件下的所有启动项
    pub items: Vec<LaunchItem>,
    /// 图标（base64 data URL，如 "data:image/png;base64,..."，无则为空字符串）
    #[serde(default)]
    pub icon: String,
}

/// 顶层分类（一级目录）
/// 注意：字段保持 snake_case（loose_items），前端按此约定读取，勿加 camelCase 重命名
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Category {
    /// 分类名（目录名）
    pub name: String,
    /// 分类目录路径
    pub path: String,
    /// 分类下的软件
    pub apps: Vec<App>,
    /// 直接放在分类下的散装启动项（无软件文件夹包装）
    pub loose_items: Vec<LaunchItem>,
}

/// 扫描结果
/// 注意：字段保持 snake_case（total_items），前端按此约定读取，勿加 camelCase 重命名
#[derive(Debug, Serialize, Deserialize)]
pub struct ScanResult {
    pub folder: String,
    pub categories: Vec<Category>,
    pub total_items: usize,
}

/// 允许直接启动的扩展名
const EXECUTABLE_EXTS: &[&str] = &["exe", "lnk", "bat", "cmd", "com", "msc"];
/// 视为链接的扩展名
const URL_EXTS: &[&str] = &["url", "website"];

/// 判断扩展名（小写、不含点）
fn ext_of(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default()
}

/// 是否为系统/隐藏干扰文件
fn is_hidden_system(name: &str) -> bool {
    name.starts_with('.')
        || name.starts_with("~$")
        || name.eq_ignore_ascii_case("desktop.ini")
        || name.eq_ignore_ascii_case("thumbs.db")
}

/// 是否为“干扰项”：卸载、帮助、文档、网站、说明类
fn is_noise(name: &str) -> bool {
    let n = name.to_lowercase();
    let noise_markers = [
        "卸载", "uninstall", "remove", "帮助", "help", "文档", "document",
        "网站", "website", "官网", "说明", "readme", "许可", "license",
        "使用问题", "问题反馈", "关于", "about", "教程", "tutorial",
        "追踪", "更新", "update", "配置工具", "设置", "reset",
    ];
    noise_markers.iter().any(|m| n.contains(m))
}

/// 判断条目类型
fn classify(path: &Path) -> String {
    if path.is_dir() {
        return "folder".to_string();
    }
    let ext = ext_of(path);
    if URL_EXTS.contains(&ext.as_str()) {
        return "url".to_string();
    }
    if ext == "bat" || ext == "cmd" {
        return "script".to_string();
    }
    if EXECUTABLE_EXTS.contains(&ext.as_str()) {
        return "app".to_string();
    }
    "file".to_string()
}

/// 判断一个文件是否是“可展示的启动项”
fn is_launchable_file(path: &Path) -> bool {
    let ext = ext_of(path);
    EXECUTABLE_EXTS.contains(&ext.as_str()) || URL_EXTS.contains(&ext.as_str())
}

/// 去掉扩展名得到显示名
fn display_name(name: &str, ext: &str) -> String {
    if ext.is_empty() {
        name.to_string()
    } else {
        name.trim_end_matches(&format!(".{}", ext)).to_string()
    }
}

/// 构建一个 LaunchItem
fn make_item(path: &Path) -> LaunchItem {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let ext = ext_of(path);
    let kind = classify(path);
    let noise = is_noise(&name);
    let dn = display_name(&name, &ext);
    LaunchItem {
        name: dn,
        path: path.to_string_lossy().to_string(),
        ext,
        kind,
        noise,
        icon: String::new(),
    }
}

/// 扫描一个目录下的所有“启动项文件”（不递归，只一层）
fn scan_loose_items(dir: &Path) -> Vec<LaunchItem> {
    let mut items: Vec<LaunchItem> = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return items,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            continue;
        }
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if is_hidden_system(name) {
            continue;
        }
        if is_launchable_file(&p) {
            items.push(make_item(&p));
        }
    }
    // 排序：非干扰项在前，然后按名称
    items.sort_by(|a, b| {
        a.noise
            .cmp(&b.noise)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    items
}

/// 扫描一个“软件目录”，返回其下所有启动项（含子目录一层，兼容 VS Studio 等嵌套）
fn scan_app_items(dir: &Path) -> Vec<LaunchItem> {
    let mut items: Vec<LaunchItem> = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return items,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if is_hidden_system(name) {
            continue;
        }
        if p.is_dir() {
            // 递归一层子目录（兼容“结合”“卸载”等嵌套），但不无限深入
            let sub = scan_loose_items(&p);
            items.extend(sub);
        } else if is_launchable_file(&p) {
            items.push(make_item(&p));
        }
    }
    items.sort_by(|a, b| {
        a.noise
            .cmp(&b.noise)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    items
}

/// 扫描顶层分类目录：识别其下软件文件夹 + 散装启动项
fn scan_category(dir: &Path) -> Category {
    let name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let mut apps: Vec<App> = Vec::new();
    let mut loose_items: Vec<LaunchItem> = Vec::new();

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => {
            return Category {
                name,
                path: dir.to_string_lossy().to_string(),
                apps,
                loose_items,
            }
        }
    };

    for entry in entries.flatten() {
        let p = entry.path();
        let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if is_hidden_system(fname) {
            continue;
        }
        if p.is_dir() {
            // 软件文件夹
            let items = scan_app_items(&p);
            if items.is_empty() {
                continue; // 空目录忽略
            }
            // 图标不在此处提取（由 scan_all 统一收集后并行提取，见 extract_icons_parallel）
            apps.push(App {
                name: fname.to_string(),
                path: p.to_string_lossy().to_string(),
                items,
                icon: String::new(),
            });
        } else if is_launchable_file(&p) {
            loose_items.push(make_item(&p));
        }
    }

    // 软件按名称排序
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    loose_items.sort_by(|a, b| {
        a.noise
            .cmp(&b.noise)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Category {
        name,
        path: dir.to_string_lossy().to_string(),
        apps,
        loose_items,
    }
}

/// 图标提取目标：文件夹卡片（categories 索引 + apps 索引）或散装项（categories 索引 + loose_items 索引）
enum IconTarget {
    App(usize, usize),
    Loose(usize, usize),
}

/// 扫描根目录，识别所有顶层分类；图标在最后统一并行提取
fn scan_all(root: &Path) -> Vec<Category> {
    let mut categories: Vec<Category> = Vec::new();
    let entries = match fs::read_dir(root) {
        Ok(e) => e,
        Err(_) => return categories,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if is_hidden_system(fname) {
            continue;
        }
        if p.is_dir() {
            categories.push(scan_category(&p));
        }
    }
    categories.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    // 收集图标提取任务：
    //  1) 文件夹卡片：取第一个非干扰项（优先非 .url）的路径；
    //  2) 散装项：逐个提取（单个快捷方式也要显示真实图标）
    let mut tasks: Vec<(IconTarget, String)> = Vec::new();
    for (ci, c) in categories.iter().enumerate() {
        for (ai, a) in c.apps.iter().enumerate() {
            let src = a
                .items
                .iter()
                .find(|it| !it.noise && it.kind != "url")
                .or_else(|| a.items.first())
                .map(|it| it.path.clone());
            if let Some(s) = src {
                tasks.push((IconTarget::App(ci, ai), s));
            }
        }
        for (li, it) in c.loose_items.iter().enumerate() {
            tasks.push((IconTarget::Loose(ci, li), it.path.clone()));
        }
    }
    let icons = extract_icons_parallel(&tasks);
    for ((target, _), icon) in tasks.iter().zip(icons.into_iter()) {
        if let Some(ic) = icon {
            match target {
                IconTarget::App(ci, ai) => categories[*ci].apps[*ai].icon = ic,
                IconTarget::Loose(ci, li) => categories[*ci].loose_items[*li].icon = ic,
            }
        }
    }
    categories
}

/// 轻量调试日志：只写异常事件（正常扫描不产生日志）；文件在 exe 同目录 scan-debug.log
pub(crate) fn debug_log(msg: &str) {
    use std::io::Write;
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("scan-debug.log");
            if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&p) {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                let _ = writeln!(f, "[{}] {}", ts, msg);
            }
        }
    }
}

/// COM 初始化守卫：线程退出前配对 CoUninitialize
struct ComGuard;

#[cfg(target_os = "windows")]
impl Drop for ComGuard {
    fn drop(&mut self) {
        unsafe {
            windows::Win32::System::Com::CoUninitialize();
        }
    }
}

/// 在当前线程初始化 COM（SHGetFileInfoW 解析 .lnk 依赖 COM 上下文，后台线程默认未初始化）。
/// 已处于其他 COM 模式（RPC_E_CHANGED_MODE）时返回 None，此时不做反初始化。
#[cfg(target_os = "windows")]
fn com_init_thread() -> Option<ComGuard> {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
    match unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) } {
        Ok(_) => Some(ComGuard),
        Err(_) => None,
    }
}

#[cfg(not(target_os = "windows"))]
fn com_init_thread() -> Option<ComGuard> {
    None
}

/// 多线程并行提取图标（线程数 = min(CPU 核数, 8)）
/// 说明：
/// 1) 每个线程先初始化 COM——SHGetFileInfoW 解析 .lnk 图标需要 COM 上下文；
/// 2) 「预热」改为单任务重试直到成功（有界，最多 10 次 × 80ms）：shell 冷态时首调会失败，
///    等它就绪再并行，而不是失败后对上百个任务逐个长等待；
/// 3) 补偿改为「有界并行重试」（最多两轮），避免把失败耗时放大成十几秒（实测旧实现最坏 12s）；
/// 4) 异常时写入 scan-debug.log（含线程 id、成败计数、耗时），便于排查 shell 图标接口故障。
fn extract_icons_parallel(tasks: &[(IconTarget, String)]) -> Vec<Option<String>> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    let n = tasks.len();
    let mut out: Vec<Option<String>> = vec![None; n];
    if n == 0 {
        return out;
    }
    let t0 = std::time::Instant::now();

    let workers = std::thread::available_parallelism()
        .map(|x| x.get())
        .unwrap_or(4)
        .clamp(1, 8);

    // 调用线程（预热所在线程）自身也初始化 COM
    let _com_main = com_init_thread();

    // —— 预热：用第一个任务重试直到成功（有界）——
    let mut warm_ok = false;
    let mut warm_rounds = 0usize;
    for r in 0..10 {
        warm_rounds = r + 1;
        if let Some(ic) = extract_icon_png(&tasks[0].1) {
            out[0] = Some(ic);
            warm_ok = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(80));
    }
    let log_workers = !warm_ok;

    // —— 第一轮：全量并行（任务 1..n）——
    let start = if n > 1 { 1 } else { n };
    if workers > 1 && n - start > 4 {
        let next = AtomicUsize::new(start);
        let collected: Mutex<Vec<(usize, Option<String>)>> =
            Mutex::new(Vec::with_capacity(n - start));
        std::thread::scope(|scope| {
            for _ in 0..workers {
                scope.spawn(|| {
                    let _com = com_init_thread();
                    if log_workers {
                        debug_log(&format!("icon: worker tid={:?}", std::thread::current().id()));
                    }
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        if i >= tasks.len() {
                            break;
                        }
                        let icon = extract_icon_png(&tasks[i].1);
                        collected.lock().unwrap().push((i, icon));
                    }
                });
            }
        });
        for (i, icon) in collected.into_inner().unwrap() {
            out[i] = icon;
        }
    } else {
        for i in start..n {
            out[i] = extract_icon_png(&tasks[i].1);
        }
    }

    // —— 补偿：仍失败的任务做有界并行重试（最多两轮），不再逐个 80ms 串行等待 ——
    for round in 0..2 {
        let failed: Vec<usize> = (0..n).filter(|&i| out[i].is_none()).collect();
        if failed.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(if round == 0 { 150 } else { 300 }));
        if workers > 1 && failed.len() > 4 {
            let next = AtomicUsize::new(0);
            let collected: Mutex<Vec<(usize, String)>> =
                Mutex::new(Vec::with_capacity(failed.len()));
            std::thread::scope(|scope| {
                for _ in 0..workers {
                    scope.spawn(|| {
                        let _com = com_init_thread();
                        loop {
                            let k = next.fetch_add(1, Ordering::Relaxed);
                            if k >= failed.len() {
                                break;
                            }
                            let i = failed[k];
                            if let Some(ic) = extract_icon_png(&tasks[i].1) {
                                collected.lock().unwrap().push((i, ic));
                            }
                        }
                    });
                }
            });
            for (i, icon) in collected.into_inner().unwrap() {
                out[i] = Some(icon);
            }
        } else {
            for &i in &failed {
                if out[i].is_none() {
                    out[i] = extract_icon_png(&tasks[i].1);
                }
            }
        }
    }

    let ok = out.iter().filter(|x| x.is_some()).count();
    let elapsed_ms = t0.elapsed().as_millis();
    // 只在「有失败」或「明显异常耗时（>3s，正常全量约 1s）」时写日志，正常扫描不产生日志文件
    if ok < n || elapsed_ms > 3000 {
        debug_log(&format!(
            "icon: n={} ok={} workers={} warm_ok={} warm_rounds={} elapsed={}ms",
            n, ok, workers, warm_ok, warm_rounds, elapsed_ms
        ));
    }
    out
}

/// 打开/启动一个条目
fn launch(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        open_with_explorer(path)
    } else {
        open_default(path)
    }
}

#[cfg(target_os = "windows")]
fn open_default(path: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    Command::new("cmd")
        .args(["/C", "start", "", path.to_str().unwrap_or("")])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW：抑制 cmd 自身的控制台窗口
        .spawn()
        .map_err(|e| format!("启动失败: {}", e))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn open_with_explorer(path: &Path) -> Result<(), String> {
    Command::new("explorer")
        .arg(path.to_str().unwrap_or(""))
        .spawn()
        .map_err(|e| format!("打开文件夹失败: {}", e))?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn open_default(path: &Path) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(path.to_str().unwrap_or(""))
        .spawn()
        .map_err(|e| format!("启动失败: {}", e))?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn open_with_explorer(path: &Path) -> Result<(), String> {
    open_default(path)
}

/// 在资源管理器中定位（选中）某个文件
/// 用 raw_arg 原样传入 /select,"路径"，避免标准参数转义破坏 explorer 的参数解析
#[cfg(target_os = "windows")]
fn reveal_in_explorer(path: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    Command::new("explorer")
        .raw_arg(format!("/select,\"{}\"", path.to_string_lossy()))
        .spawn()
        .map_err(|e| format!("打开位置失败: {}", e))?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn reveal_in_explorer(path: &Path) -> Result<(), String> {
    let dir = path.parent().unwrap_or(path);
    open_with_explorer(dir)
}

/// 显示文件/文件夹的系统「属性」对话框（Windows: ShellExecuteExW + properties 动词）
#[cfg(target_os = "windows")]
fn show_properties(path: &str) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_INVOKEIDLIST, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;

    // 属性对话框依赖 shell/COM 上下文
    let _com = com_init_thread();

    let file: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let verb: Vec<u16> = "properties".encode_utf16().chain(std::iter::once(0)).collect();

    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_INVOKEIDLIST; // 「属性」动词需要 INVOKEIDLIST
    info.lpVerb = PCWSTR(verb.as_ptr());
    info.lpFile = PCWSTR(file.as_ptr());
    info.nShow = SW_SHOW.0;

    unsafe {
        ShellExecuteExW(&mut info).map_err(|e| format!("打开属性失败: {}", e))?;
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn show_properties(_path: &str) -> Result<(), String> {
    Err("当前平台不支持属性对话框".to_string())
}

/// 解析 .lnk 快捷方式指向的目标路径（失败返回 None；用于「打开文件位置」与系统行为保持一致）
#[cfg(target_os = "windows")]
fn resolve_lnk_target(path: &Path) -> Option<String> {
    use windows::core::{ComInterface, PCWSTR};
    use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW;
    use windows::Win32::System::Com::{CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER, STGM_READ};
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink, SLGP_RAWPATH};

    let _com = com_init_thread();
    let wide: Vec<u16> = path.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        let pf: IPersistFile = link.cast().ok()?;
        pf.Load(PCWSTR(wide.as_ptr()), STGM_READ).ok()?;
        let mut buf = [0u16; 1024];
        let mut fd: WIN32_FIND_DATAW = std::mem::zeroed();
        link.GetPath(&mut buf, &mut fd, SLGP_RAWPATH.0 as u32).ok()?;
        let len = buf.iter().position(|&c| c == 0).unwrap_or(0);
        if len == 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buf[..len]))
    }
}

/// 打开系统「选择文件夹」对话框（Windows: IFileOpenDialog + FOS_PICKFOLDERS）
#[cfg(target_os = "windows")]
fn pick_folder_dialog() -> Result<Option<String>, String> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{
        FileOpenDialog, IFileOpenDialog, IShellItem, SIGDN_FILESYSPATH, FOS_FORCEFILESYSTEM,
        FOS_PATHMUSTEXIST, FOS_PICKFOLDERS,
    };

    unsafe {
        // 初始化 COM（若已初始化会返回 RPC_E_CHANGED_MODE，可忽略）
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let result = (|| -> Result<Option<String>, String> {
            let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_ALL)
                .map_err(|e| format!("创建对话框失败: {}", e))?;

            let mut options = dialog.GetOptions().map_err(|e| format!("读取选项失败: {}", e))?;
            options |= FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST;
            dialog.SetOptions(options).map_err(|e| format!("设置选项失败: {}", e))?;

            // 显示对话框：用户取消返回 Err(HRESULT 0x800704C7)，视为取消
            match dialog.Show(None) {
                Ok(()) => {}
                Err(_) => return Ok(None), // 用户取消
            }

            let item: IShellItem = dialog.GetResult().map_err(|e| format!("获取结果失败: {}", e))?;
            let pwstr = item
                .GetDisplayName(SIGDN_FILESYSPATH)
                .map_err(|e| format!("获取路径失败: {}", e))?;
            let path = pwstr.to_string().map_err(|e| format!("路径转换失败: {}", e))?;
            Ok(Some(path))
        })();

        CoUninitialize();
        result
    }
}

#[cfg(not(target_os = "windows"))]
fn pick_folder_dialog() -> Result<Option<String>, String> {
    Err("当前平台不支持文件夹选择对话框".to_string())
}

/// 配置路径（配置与 exe 同目录，不写入 C 盘）
/// 例如 exe 在 D:\Apps\FolderLaunchpad.exe，则配置为 D:\Apps\launcher-config.json
fn config_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            return dir.join("launcher-config.json");
        }
    }
    PathBuf::from("launcher-config.json")
}

/// 应用配置（持久化到 exe 同目录的 launcher-config.json）
#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub folder: Option<String>,
    #[serde(default)]
    pub favorites: Vec<FavoriteItem>,
    /// 启动时是否自动加载上次使用的文件夹
    #[serde(default = "default_true")]
    pub auto_load: bool,
    /// 主题：light / dark（旧配置缺少该字段时使用默认值）
    #[serde(default = "default_theme")]
    pub theme: String,
    /// 正常按钮的启动点击次数：single / double
    #[serde(default = "default_click")]
    pub click_normal: String,
    /// 灰色弱化按钮的启动点击次数：single / double
    #[serde(default = "default_click")]
    pub click_weak: String,
}

fn default_true() -> bool {
    true
}

fn default_theme() -> String {
    "light".to_string()
}

fn default_click() -> String {
    "single".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            folder: None,
            favorites: Vec::new(),
            auto_load: true,
            theme: default_theme(),
            click_normal: default_click(),
            click_weak: default_click(),
        }
    }
}

/// 收藏项
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteItem {
    pub name: String,
    pub path: String,
    pub kind: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub category: String,
}

fn read_config() -> AppConfig {
    let p = config_path();
    match fs::read_to_string(&p) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

fn write_config(cfg: &AppConfig) -> Result<(), String> {
    let p = config_path();
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {}", e))?;
    }
    let json = serde_json::to_string_pretty(cfg).map_err(|e| format!("序列化配置失败: {}", e))?;
    fs::write(&p, json).map_err(|e| format!("保存配置失败: {}", e))?;
    Ok(())
}

/// 扫描结果缓存路径（与配置同目录，随时可删）
fn scan_cache_path() -> PathBuf {
    let mut p = config_path();
    p.set_file_name("launcher-scan-cache.json");
    p
}

/// 序列化缓存用的借用结构（避免为了写缓存克隆整个扫描结果）
#[derive(Serialize)]
struct ScanCacheRef<'a> {
    folder: &'a str,
    result: &'a ScanResult,
}

/// 读取缓存用的结构
#[derive(Deserialize)]
struct ScanCacheLoaded {
    folder: String,
    result: ScanResult,
}

/// 把扫描结果写入缓存（尽力而为：失败只影响下次启动的「秒开」，不影响本次结果）
fn write_scan_cache(result: &ScanResult) {
    if let Ok(json) = serde_json::to_string(&ScanCacheRef {
        folder: &result.folder,
        result,
    }) {
        let _ = fs::write(scan_cache_path(), json);
    }
}

// ---------- Tauri 命令 ----------

/// 扫描指定文件夹（两级分类）
/// async + spawn_blocking：扫描含图标提取属于耗时操作，放到后台线程执行，
/// 避免占用主线程导致窗口卡死（同步命令会阻塞主线程）
#[tauri::command]
async fn cmd_scan_folder(folder: String) -> Result<ScanResult, String> {
    match tauri::async_runtime::spawn_blocking(move || scan_folder_blocking(folder)).await {
        Ok(result) => result,
        Err(e) => Err(format!("扫描任务异常: {}", e)),
    }
}

/// 扫描的同步实现（在后台线程中执行）
fn scan_folder_blocking(folder: String) -> Result<ScanResult, String> {
    let p = PathBuf::from(&folder);
    if !p.exists() {
        return Err(format!("文件夹不存在: {}", folder));
    }
    if !p.is_dir() {
        return Err(format!("路径不是文件夹: {}", folder));
    }
    let categories = scan_all(&p);
    let total_items = categories
        .iter()
        .map(|c| c.apps.iter().map(|a| a.items.len()).sum::<usize>() + c.loose_items.len())
        .sum();
    let result = ScanResult {
        folder,
        categories,
        total_items,
    };
    // 写入磁盘缓存：下次启动可直接用缓存秒开
    write_scan_cache(&result);
    Ok(result)
}

/// 打开/启动条目
#[tauri::command]
fn cmd_open_item(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    launch(&p)
}

/// 在资源管理器中打开一个文件夹
#[tauri::command]
fn cmd_open_folder(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    open_with_explorer(&p)
}

/// 在资源管理器中打开条目：文件夹直接打开；文件则打开其所在文件夹
#[tauri::command]
fn cmd_open_in_explorer(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if p.is_dir() {
        open_with_explorer(&p)
    } else {
        let dir = p.parent().map(|d| d.to_path_buf()).unwrap_or(p);
        open_with_explorer(&dir)
    }
}

/// 「打开文件位置」：打开对象上一级所在的文件夹（不做快捷方式解析）
/// - 文件 → 其所在文件夹
/// - 文件夹 → 其父目录（如 …\桌面\游戏\某游戏 → …\桌面\游戏）
#[tauri::command]
fn cmd_open_file_location(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    let parent = p
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .map(|d| d.to_path_buf())
        .ok_or_else(|| format!("无法定位上一级位置: {}", path))?;
    open_with_explorer(&parent)
}

/// 「打开原文件位置」：解析 .lnk 快捷方式打开其指向的原软件位置（文件→所在文件夹并选中；文件夹→打开本身）
/// 非快捷方式（或解析/目标不存在）时，回退为对象自身位置：文件夹→打开本身；文件→打开所在文件夹
#[tauri::command]
fn cmd_open_original_location(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    #[cfg(target_os = "windows")]
    {
        if p.extension().map(|e| e.eq_ignore_ascii_case("lnk")).unwrap_or(false) {
            if let Some(target) = resolve_lnk_target(&p) {
                let t = PathBuf::from(&target);
                if t.exists() {
                    return if t.is_dir() {
                        open_with_explorer(&t)
                    } else {
                        reveal_in_explorer(&t)
                    };
                }
            }
        }
    }
    if p.is_dir() {
        open_with_explorer(&p)
    } else {
        let dir = p.parent().map(|d| d.to_path_buf()).unwrap_or(p);
        open_with_explorer(&dir)
    }
}

/// 显示条目的系统「属性」对话框
#[tauri::command]
fn cmd_show_properties(path: String) -> Result<(), String> {
    show_properties(&path)
}

/// 获取上次保存的文件夹
#[tauri::command]
fn cmd_get_saved_folder() -> Result<Option<String>, String> {
    Ok(read_config().folder)
}

/// 保存文件夹到配置
#[tauri::command]
fn cmd_save_folder(folder: String) -> Result<(), String> {
    let mut cfg = read_config();
    cfg.folder = Some(folder);
    write_config(&cfg)
}

/// 获取收藏列表
#[tauri::command]
fn cmd_get_favorites() -> Result<Vec<FavoriteItem>, String> {
    Ok(read_config().favorites)
}

/// 添加收藏
#[tauri::command]
fn cmd_add_favorite(item: FavoriteItem) -> Result<Vec<FavoriteItem>, String> {
    let mut cfg = read_config();
    if !cfg.favorites.iter().any(|f| f.path == item.path && f.name == item.name) {
        cfg.favorites.push(item);
    }
    write_config(&cfg)?;
    Ok(cfg.favorites)
}

/// 移除收藏（按路径 + 名称匹配）
#[tauri::command]
fn cmd_remove_favorite(name: String, path: String) -> Result<Vec<FavoriteItem>, String> {
    let mut cfg = read_config();
    cfg.favorites.retain(|f| !(f.path == path && f.name == name));
    write_config(&cfg)?;
    Ok(cfg.favorites)
}

/// 打开系统的文件夹选择对话框，返回用户选中的路径（取消则返回 None）
#[tauri::command]
fn cmd_pick_folder() -> Result<Option<String>, String> {
    pick_folder_dialog()
}

/// 应用自身信息（设置页展示用）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub exe_path: String,
    pub config_path: String,
    pub data_dir: String,
}

/// 返回程序自身信息（版本、exe 路径、配置与缓存位置）
#[tauri::command]
fn cmd_get_app_info() -> AppInfo {
    let exe = std::env::current_exe().ok();
    let exe_path = exe
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let data_dir = exe
        .as_ref()
        .and_then(|p| p.parent())
        .map(|d| d.join("webview-data").to_string_lossy().to_string())
        .unwrap_or_default();
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        exe_path,
        config_path: config_path().to_string_lossy().to_string(),
        data_dir,
    }
}

/// 读取「启动时自动加载上次文件夹」开关
#[tauri::command]
fn cmd_get_auto_load() -> Result<bool, String> {
    Ok(read_config().auto_load)
}

/// 设置「启动时自动加载上次文件夹」开关
#[tauri::command]
fn cmd_set_auto_load(enabled: bool) -> Result<(), String> {
    let mut cfg = read_config();
    cfg.auto_load = enabled;
    write_config(&cfg)
}

/// 用户偏好集合（设置页一次性读取）
#[derive(Debug, Serialize)]
pub struct Settings {
    pub auto_load: bool,
    pub theme: String,
    pub click_normal: String,
    pub click_weak: String,
}

/// 读取全部用户偏好
#[tauri::command]
fn cmd_get_settings() -> Settings {
    let cfg = read_config();
    Settings {
        auto_load: cfg.auto_load,
        theme: cfg.theme,
        click_normal: cfg.click_normal,
        click_weak: cfg.click_weak,
    }
}

/// 设置主题（light / dark）
#[tauri::command]
fn cmd_set_theme(theme: String) -> Result<(), String> {
    if theme != "light" && theme != "dark" {
        return Err(format!("不支持的主题: {}", theme));
    }
    let mut cfg = read_config();
    cfg.theme = theme;
    write_config(&cfg)
}

/// 设置「启动点击次数」：target = normal（正常按钮）/ weak（灰色弱化按钮）；mode = single / double
#[tauri::command]
fn cmd_set_click_mode(target: String, mode: String) -> Result<(), String> {
    if mode != "single" && mode != "double" {
        return Err(format!("不支持的点击模式: {}", mode));
    }
    let mut cfg = read_config();
    match target.as_str() {
        "normal" => cfg.click_normal = mode,
        "weak" => cfg.click_weak = mode,
        _ => return Err(format!("不支持的按钮类型: {}", target)),
    }
    write_config(&cfg)
}

/// 读取上次扫描的缓存结果（folder 匹配才返回，用于启动「秒开」；随后仍会后台校验更新）
#[tauri::command]
async fn cmd_load_scan_cache(folder: String) -> Result<Option<ScanResult>, String> {
    match tauri::async_runtime::spawn_blocking(move || {
        let data = fs::read_to_string(scan_cache_path()).ok()?;
        let cache: ScanCacheLoaded = serde_json::from_str(&data).ok()?;
        if cache.folder == folder {
            Some(cache.result)
        } else {
            None
        }
    })
    .await
    {
        Ok(v) => Ok(v),
        Err(e) => Err(format!("读取缓存任务异常: {}", e)),
    }
}

/// 在资源管理器中定位文件（「打开文件位置」）
/// 对 .lnk 快捷方式：优先定位其指向的目标（与系统右键「打开文件位置」一致）；
/// 解析失败或目标不存在时，退化为定位快捷方式本身；文件不存在时退化为打开其所在文件夹
#[tauri::command]
fn cmd_reveal_file(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    #[cfg(target_os = "windows")]
    {
        if p.extension().map(|e| e.eq_ignore_ascii_case("lnk")).unwrap_or(false) {
            if let Some(target) = resolve_lnk_target(&p) {
                let t = PathBuf::from(&target);
                if t.exists() {
                    return reveal_in_explorer(&t);
                }
            }
        }
    }
    if p.exists() {
        reveal_in_explorer(&p)
    } else {
        let dir = p.parent().map(|d| d.to_path_buf()).unwrap_or(p);
        open_with_explorer(&dir)
    }
}

/// 清理缓存结果
#[derive(Debug, Serialize)]
pub struct ClearCacheResult {
    /// 实际释放的字节数
    pub freed_bytes: u64,
    /// 因被占用等原因未能清理的项数
    pub skipped: u64,
}

/// exe 所在目录（所有数据的锚点）
fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// 递归计算文件/目录体积
fn path_size(p: &Path) -> u64 {
    if p.is_dir() {
        let mut total = 0u64;
        if let Ok(rd) = fs::read_dir(p) {
            for e in rd.flatten() {
                let ep = e.path();
                if ep.is_dir() {
                    total += path_size(&ep);
                } else if let Ok(md) = e.metadata() {
                    total += md.len();
                }
            }
        }
        total
    } else {
        p.metadata().map(|m| m.len()).unwrap_or(0)
    }
}

/// 尽力删除目录：返回（实际释放字节数, 是否完全成功）
/// 运行中的 WebView2 会占用部分缓存文件，被占用的部分无法删除，这里如实统计
fn remove_dir_best_effort(p: &Path) -> (u64, bool) {
    if !p.exists() {
        return (0, true);
    }
    let before = path_size(p);
    match fs::remove_dir_all(p) {
        Ok(_) => (before, true),
        Err(_) => {
            let after = if p.exists() { path_size(p) } else { 0 };
            (before.saturating_sub(after), false)
        }
    }
}

fn clear_cache_blocking() -> ClearCacheResult {
    let mut freed = 0u64;
    let mut skipped = 0u64;
    let base = exe_dir();

    // 1) 扫描缓存 + 异常日志（秒开相关，删掉后下次启动回退为完整扫描）
    for name in ["launcher-scan-cache.json", "scan-debug.log"] {
        let f = base.join(name);
        if let Ok(md) = fs::metadata(&f) {
            if fs::remove_file(&f).is_ok() {
                freed += md.len();
            } else {
                skipped += 1;
            }
        }
    }

    // 2) 浏览器内核磁盘缓存（数据目录中的缓存类文件夹；组件与用户数据不动）
    let wv = base.join("webview-data").join("EBWebView");
    let targets = [
        wv.join("GrShaderCache"),
        wv.join("ShaderCache"),
        wv.join("GraphiteDawnCache"),
        wv.join("Default").join("Cache"),
        wv.join("Default").join("Code Cache"),
        wv.join("Default").join("GPUCache"),
        wv.join("Default").join("DawnWebGPUCache"),
        wv.join("Default").join("DawnGraphiteCache"),
    ];
    for t in targets {
        if t.exists() {
            let (size, ok) = remove_dir_best_effort(&t);
            freed += size;
            if !ok {
                skipped += 1;
            }
        }
    }

    ClearCacheResult { freed_bytes: freed, skipped }
}

/// 清理缓存：扫描缓存 + 浏览器磁盘缓存（异步执行，避免阻塞窗口）
#[tauri::command]
async fn cmd_clear_cache() -> Result<ClearCacheResult, String> {
    match tauri::async_runtime::spawn_blocking(clear_cache_blocking).await {
        Ok(r) => Ok(r),
        Err(e) => Err(format!("清理任务异常: {}", e)),
    }
}

/// 构建 Tauri 应用（在 main.rs 调用），集中注册所有命令
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // 手动创建主窗口，把 WebView2 用户数据目录固定在 exe 同目录下的
            // webview-data 文件夹，避免默认写入 C 盘（%LOCALAPPDATA%\{identifier}\EBWebView）
            let mut builder = tauri::WindowBuilder::new(
                app,
                "main",
                tauri::WindowUrl::App("index.html".into()),
            )
            .title("Folder Launchpad")
            .inner_size(1080.0, 720.0)
            .min_inner_size(720.0, 480.0)
            .resizable(true)
            .center();

            if let Ok(exe) = std::env::current_exe() {
                if let Some(dir) = exe.parent() {
                    builder = builder.data_directory(dir.join("webview-data"));
                }
            }

            let _window = builder.build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cmd_scan_folder,
            cmd_open_item,
            cmd_open_folder,
            cmd_open_in_explorer,
            cmd_open_file_location,
            cmd_open_original_location,
            cmd_show_properties,
            cmd_get_saved_folder,
            cmd_save_folder,
            cmd_get_favorites,
            cmd_add_favorite,
            cmd_remove_favorite,
            cmd_pick_folder,
            cmd_get_app_info,
            cmd_get_auto_load,
            cmd_set_auto_load,
            cmd_get_settings,
            cmd_set_theme,
            cmd_set_click_mode,
            cmd_reveal_file,
            cmd_load_scan_cache,
            cmd_clear_cache
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
