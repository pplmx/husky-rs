use std::fs::File;
use std::io::Read;
use std::{
    env::{self, var_os},
    fs, io,
    io::{BufRead, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;

// 定义错误类型
#[derive(Error, Debug)]
enum HuskyError {
    #[error(".git directory was not found in '{0}' or its parent directories")]
    GitDirNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Environment variable error: {0}")]
    Env(#[from] env::VarError),

    #[error("User hooks directory is invalid: '{0:?}'")]
    InvalidUserHooksDir(PathBuf),

    #[error("User hook script is empty: '{0:?}'")]
    EmptyUserHook(PathBuf),
}

// 定义 Result 类型别名
type Result<T> = std::result::Result<T, HuskyError>;

// 查找 Git 目录
fn resolve_git_dir() -> Result<PathBuf> {
    let dir = env::var("OUT_DIR")?;
    let mut dir = PathBuf::from(dir);
    if !dir.has_root() {
        dir = fs::canonicalize(&dir)?;
    }

    loop {
        let git_dir = dir.join(".git");
        if git_dir.is_dir() {
            return Ok(git_dir);
        } else if git_dir.is_file() {
            return read_git_submodule(git_dir);
        }

        if !dir.pop() {
            return Err(HuskyError::GitDirNotFound(
                env::var("OUT_DIR").unwrap_or_default(),
            ));
        }
    }
}

// 处理 Git 子模块情况
fn read_git_submodule(git_file: PathBuf) -> Result<PathBuf> {
    let mut content = String::new();
    File::open(git_file)?.read_to_string(&mut content)?;
    let newlines = ['\n', '\r'];
    let git_dir = PathBuf::from(content.trim_end_matches(&newlines));
    if !git_dir.is_dir() {
        return Err(HuskyError::GitDirNotFound(git_dir.display().to_string()));
    }
    Ok(git_dir)
}

// 检查钩子是否已存在
fn hook_already_exists(hook: &Path) -> bool {
    if let Ok(f) = File::open(hook) {
        let ver_line = io::BufReader::new(f)
            .lines()
            .nth(2)
            .unwrap_or(Ok(String::new()))
            .ok();
        if let Some(ver_line) = ver_line {
            return ver_line.contains("This hook was set by husky-rs");
        }
    }
    false
}

// 安装用户钩子
fn install_user_hooks() -> Result<()> {
    let git_dir = resolve_git_dir()?; // 获取 .git 目录

    // 获取项目根目录，并处理没有父目录的情况
    let project_root = git_dir
        .parent()
        .ok_or_else(|| HuskyError::GitDirNotFound(git_dir.display().to_string()))?;

    // 拼接 .husky-rs/hooks 路径
    let user_hooks_dir = project_root.join(".husky-rs").join("hooks");

    // 输出调试信息
    println!("User hooks directory path: {:?}", user_hooks_dir);

    // 检查用户钩子目录是否存在
    if !user_hooks_dir.is_dir() {
        return Err(HuskyError::InvalidUserHooksDir(user_hooks_dir));
    }

    // 拼接 git hooks 目录
    let hooks_dir = git_dir.join("hooks");

    // 如果 git hooks 目录不存在，则创建
    if !hooks_dir.exists() {
        fs::create_dir(&hooks_dir)?;
    }

    // 遍历用户定义的钩子文件并安装
    for hook in fs::read_dir(&user_hooks_dir)?
        .filter_map(|e| e.ok()) // 忽略无法读取的条目
        .filter(is_executable_file) // 只处理可执行文件
        .map(|e| e.path())
    // 获取路径
    {
        install_user_hook(&hook, &hooks_dir)?; // 安装钩子
    }

    Ok(())
}

// 安装用户钩子文件
fn install_user_hook(src: &Path, dst_dir: &Path) -> Result<()> {
    let dst = dst_dir.join(src.file_name().unwrap());

    if hook_already_exists(&dst) {
        return Ok(());
    }

    let mut lines: Vec<String> = io::BufReader::new(File::open(src)?)
        .lines()
        .map(|line| line.map_err(HuskyError::from)) // 将 io::Error 转换为 HookError
        .collect::<Result<_>>()?;

    if lines.is_empty() {
        return Err(HuskyError::EmptyUserHook(src.to_owned()));
    }

    // 添加 husky-rs 版本注释
    if !lines[0].starts_with("#!") {
        lines.insert(0, "#".to_string());
    }
    lines.insert(1, "#".to_string());
    lines.insert(
        2,
        format!(
            "# This hook was set by husky-rs v{}: {}",
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_HOMEPAGE")
        ),
    );

    let mut f = io::BufWriter::new(create_executable_file(&dst)?);
    for line in lines {
        writeln!(f, "{}", line)?;
    }

    Ok(())
}

// 根据操作系统创建可执行文件
#[cfg(not(target_os = "windows"))]
fn create_executable_file(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o755)
        .open(path)
}

#[cfg(target_os = "windows")]
fn create_executable_file(path: &Path) -> io::Result<File> {
    File::create(path)
}

// 判断文件是否可执行 (Unix 和 Windows 适配)
#[cfg(not(target_os = "windows"))]
fn is_executable_file(entry: &fs::DirEntry) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let metadata = entry.metadata().ok()?;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(target_os = "windows")]
fn is_executable_file(entry: &fs::DirEntry) -> bool {
    entry.file_type().map(|ft| ft.is_file()).unwrap_or(false)
}

// 主函数
fn main() -> Result<()> {
    // 检查是否设置了不安装钩子的环境变量
    if var_os("CARGO_HUSKY_DONT_INSTALL_HOOKS").is_some() {
        eprintln!("Warning: Found '$CARGO_HUSKY_DONT_INSTALL_HOOKS' in env, not doing anything!");
        return Ok(());
    }

    // 执行用户钩子安装
    match install_user_hooks() {
        Err(e @ HuskyError::GitDirNotFound(_)) => {
            // 如果没有找到 Git 目录，输出警告但不中断程序
            eprintln!("Warning: {:?}", e);
            Ok(())
        }
        result => result,
    }
}
