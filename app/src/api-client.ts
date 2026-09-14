import type { components, paths } from "@jotline/contracts";
import createClient from "openapi-fetch";

export type NoteResponse = components["schemas"]["NoteResponse"];
export type Project = components["schemas"]["Project"];
export type SearchHit = components["schemas"]["SearchHit"];
export type ErrorEnvelope = components["schemas"]["ErrorEnvelope"];
export type CreateNoteRequest = components["schemas"]["CreateNoteRequest"];

/** 领域 API 基址（dev 默认真实主进程 :4765；连 Mock 经 `VITE_API_BASE_URL` 切 :4766）。 */
export const baseUrl: string =
  import.meta.env.VITE_API_BASE_URL ?? "http://127.0.0.1:4765";

export const client = createClient<paths>({ baseUrl });

/** dev 固定 Bearer token（Roadmap 总则 4；经 `VITE_DEV_TOKEN` 注入）。 */
export const DEV_TOKEN: string = import.meta.env.VITE_DEV_TOKEN ?? "dev-token";

const authHeaders = (): Record<string, string> => ({
  Authorization: `Bearer ${DEV_TOKEN}`,
});

// —— 薄封装（Phase-2 单页面三件套所需最小面；全部走契约生成类型）——

export async function listProjects() {
  return client.GET("/api/projects", { headers: authHeaders() });
}

export async function createNote(body: CreateNoteRequest) {
  return client.POST("/api/notes", { body, headers: authHeaders() });
}

export async function readNote(noteId: string) {
  return client.GET("/api/notes/{note_id}", {
    params: { path: { note_id: noteId } },
    headers: authHeaders(),
  });
}

export async function searchNotes(query: { q: string; project_id?: string }) {
  return client.GET("/api/search", {
    params: { query },
    headers: authHeaders(),
  });
}

/** 错误信封 → 人读消息（422 时含字段定位）。 */
export function errorDetail(error: ErrorEnvelope | undefined): string {
  if (!error) return "未知错误";
  const fields = error.detail
    ?.map((d) => `${d.field}：${d.message}`)
    .join("；");
  return fields ? `${error.message}（${fields}）` : error.message;
}
