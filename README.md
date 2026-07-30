# Trove2

A desktop toolbox application built with [Tauri v2](https://tauri.app/) and [Svelte 5](https://svelte.dev/).

## Features

- **Video Concat** - Merge multiple video files into one, powered by FFmpeg
  - Drag-and-drop file selection and reordering
  - Real-time progress tracking with cancel support
  - Video metadata display (duration, resolution, codec)

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) >= 18
- [FFmpeg](https://ffmpeg.org/) installed and available in PATH

## Development

```bash
# Install dependencies
npm install

# Start dev server with hot-reload
npm run tauri dev

# Build for production
npm run tauri build
```

## Tech Stack

- **Frontend**: Svelte 5 + TypeScript + Vite
- **Backend**: Rust (Tauri v2)
- **Video Processing**: FFmpeg (via CLI invocation)

## License

[MIT](LICENSE)
