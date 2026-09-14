import react from "@vitejs/plugin-react";
import { configDefaults, defineConfig } from "vitest/config";

// 前端 dev server（Roadmap 总则 4：127.0.0.1:1420，Phase-2 起启用）
export default defineConfig({
  plugins: [react()],
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
  },
  test: {
    // 排除 macOS 外置卷的 AppleDouble 元数据文件（._*，.gitignore 同列）
    exclude: [...configDefaults.exclude, "**/._*"],
  },
});
