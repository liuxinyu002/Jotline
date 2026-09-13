import createClient from "openapi-fetch";
import type { paths } from "@jotline/contracts";

/** 领域 API 客户端（Phase-11 接入前端工程；Mock 模式切换BaseUrl 127.0.0.1:4766）。 */
export const client = createClient<paths>({ baseUrl: "http://127.0.0.1:4765" });

/** dev 固定 Bearer token（Roadmap 总则 4）。 */
export const DEV_TOKEN = "dev-token";
