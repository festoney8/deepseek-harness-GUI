# DeepSeek Harness GUI

**超 级 轻 量 的 deepseek harness 桌面端应用，安装包只要 3 MB，便携版只要 9 MB！**

---

![](public/1.png)

## 安装

- 环境准备：目前只支持 Windows 系统，需安装 [**Node.js >= v24**](https://nodejs.org/zh-cn/download)
- 前往 [**Release**](https://github.com/festoney8/deepseek-harness-GUI/releases/) 下载安装包，文件名包含 setup.exe 是安装版，portable.exe 是便携版免安装

## 介绍

- 本项目是基于 Tauri 构建的 [deepseek-harness](https://github.com/deepseek-ai/deepseek-harness) 轻量启动器，保留原始 DSH 功能，提供桌面版 APP 体验
- 适配文件下载、图片拖拽、剪贴板等操作
- 支持将应用最小化到通知栏
- 支持 Ctrl+滚轮 或 Ctrl +/- 调节页面缩放
- 本项目使用 worker 模式管理 DSH，支持应用中安装/更新 DSH
- 只要官方启动命令不变，本项目就能持续使用

## 日志

- **查看终端输出** 按钮会显示成功运行 DSH 之前的终端输出，可点开查看报错
- 日志文件目录在 `C:\Users\<用户名>\AppData\Local\deepseek-harness-gui\logs`，可用于检查运行问题
- 启动应用时会用时间戳创建日志文件夹，7 天之前的日志会自动清理

## 自行构建

### 环境依赖

- windows 环境
- node.js >= v24
- pnpm
- 系统应支持 webview2

### 常用命令

```shell
# clone 项目
git clone https://github.com/festoney8/deepseek-harness-GUI
cd deepseek-harness-GUI

# 安装依赖
pnpm i

# 开发模式
pnpm tauri dev

# 构建安装包，产物路径 src-tauri\target\release\bundle\nsis
pnpm build:installer

# 构建便携版，产物路径 src-tauri\target\release\bundle\portable
pnpm build:portable
```
