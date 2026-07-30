# Trove2

基于 [Tauri v2](https://tauri.app/) 和 [Svelte 5](https://svelte.dev/) 构建的桌面工具箱应用。

## 功能

- **视频拼接** - 将多个视频文件合并为一个，基于 FFmpeg
  - 拖拽选择文件并自由排序
  - 实时进度显示，支持取消操作
  - 视频信息展示（时长、分辨率、编码格式）

## 环境要求

- [Rust](https://rustup.rs/)（stable）
- [Node.js](https://nodejs.org/) >= 18
- [FFmpeg](https://ffmpeg.org/) 已安装并在 PATH 中可用

## 开发

```bash
# 安装依赖
npm install

# 启动开发服务器（热更新）
npm run tauri dev

# 构建生产版本
npm run tauri build
```

## 技术栈

- **前端**: Svelte 5 + TypeScript + Vite
- **后端**: Rust (Tauri v2)
- **视频处理**: FFmpeg（通过命令行调用）

## 开源协议

[MIT](LICENSE)
