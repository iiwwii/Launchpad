# 启动台（Launchpad）

一个 Windows 桌面启动台，灵感来自 Mac 的 Launchpad 和 [25H/Maya](https://github.com/25H/Maya)。全屏 Mica 毛玻璃 + 图标网格，**用文件系统当数据源**——所见即所得。
<img width="3200" height="2000" alt="image" src="https://github.com/user-attachments/assets/2765fc85-c657-4288-ab23-112953221011" />

## 特点

- **文件系统驱动**：绑定一个根文件夹（默认 `桌面/启动台`），文件夹里的文件＝启动项、子文件夹＝分类。在资源管理器里整理，启动台自动同步。
- **Mac 启动台风格**：全屏、Win11 Mica 背景、图标网格、顶部搜索、点空白/ESC 收起。
- **真实图标**：用 Win32 API 提取 `.exe`/`.lnk`/`.url` 等的真实大图标（256px），透明镂空正确处理，磁盘缓存。
- **二级文件夹框**：点文件夹弹出 Mac 风格浮动框，支持翻页 / 滚轮 / 触摸板 / 图标拖拽排序。

## 功能

- 点击启动真实程序（`ShellExecuteW`，自动解引用 `.lnk`）
- 实时搜索（跨所有分类）
- 拖拽排序、拖入文件自动建 `.lnk` 快捷方式（不动原文件）
- 右键菜单：启动 / 重命名 / 删除快捷方式 / 打开所在文件夹
- 全局快捷键呼出（默认 `Ctrl+Alt+Space`，设置里**按键录制**自定义）
- 开机自启、失焦自动收起、文件变化自动刷新
- 设置面板：根路径 / 快捷键 / 主题 / 毛玻璃 / 列数 / 开机自启
- 系统托盘图标（任务栏不显示，避免闪动）

## 技术栈

- **Tauri 2**（Rust 后端 + WebView2 前端）
- **React 18 + TypeScript + Vite**
- **dnd-kit**（拖拽排序）
- **Win32 API**（`PrivateExtractIcons` 图标提取、`IShellLinkW` .lnk 解析/创建、`ShellExecuteW` 启动）
- **window-vibrancy**（Mica 毛玻璃）、**notify**（文件监听）

## 开发

需要 Node.js 和 Rust 工具链。

```bash
npm install
npm run tauri dev
```

## 构建

```bash
npm run tauri build
```

生成 `src-tauri/target/release/launchpad.exe`（便携版，约 9.5 MB，Win10/11 自带 WebView2 即可直接运行）。

> msi 安装包打包需要联网下载 WiX 工具，网络不通时只生成便携 exe。

## 使用

1. 运行 `launchpad.exe`（首次运行自动在**桌面**创建「启动台」文件夹）。
2. 往「桌面/启动台」里放快捷方式/程序 → 首页显示图标，点击启动。
3. 在该文件夹下建子文件夹 → 作为**分类**（点文件夹卡片弹出浮动框，里面显示该分类的快捷方式）。
4. 全局快捷键 `Ctrl+Alt+Space` 呼出/收起（设置里可改）；ESC 或点空白收起。
5. 拖拽图标排序、拖文件进来建快捷方式、右键图标重命名/删除。

## 数据存储

- **图标排序/改名**：存在每个文件夹下的隐藏 `.launchpad.json`（跟随文件夹走，可移植/备份）。
- **全局配置**（根路径/快捷键/主题等）：存在系统 app config 目录。
- **图标缓存**：存在系统 app cache 目录。

> 仅支持 Windows（大量 Win32 调用）。

## License

MIT
