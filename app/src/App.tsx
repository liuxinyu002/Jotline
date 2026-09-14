import type { FormEvent } from "react";
import { useCallback, useEffect, useState } from "react";
import {
  createNote,
  errorDetail,
  listProjects,
  type NoteResponse,
  type Project,
  type SearchHit,
  searchNotes,
} from "./api-client";
import { useNoteStream } from "./use-note-stream";

/**
 * Phase-2 单页面三件套（链路验证骨架，显式豁免 DESIGN.md 设计系统）：
 * 笔记列表（SSE 驱动 + 本地创建累积）+ 创建表单（项目下拉）+ 搜索框（snippet 结果）。
 */
export function App() {
  // —— 项目下拉 ——
  const [projects, setProjects] = useState<Project[]>([]);
  const [projectsError, setProjectsError] = useState<string | null>(null);
  const [selectedProject, setSelectedProject] = useState("");

  // —— 笔记列表（Phase-2 契约无列表端点：初始空态，SSE + 本地创建累积；按 id 去重）——
  const [notes, setNotes] = useState<NoteResponse[]>([]);

  // —— 创建表单 ——
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  // —— 搜索 ——
  const [query, setQuery] = useState("");
  const [searching, setSearching] = useState(false);
  const [searchResults, setSearchResults] = useState<SearchHit[] | null>(null);
  const [searchError, setSearchError] = useState<string | null>(null);

  const addNote = useCallback((note: NoteResponse) => {
    setNotes((prev) =>
      prev.some((n) => n.id === note.id) ? prev : [note, ...prev],
    );
  }, []);

  const streamStatus = useNoteStream(addNote);

  useEffect(() => {
    listProjects().then(({ data, error }) => {
      if (data) {
        setProjects(data.items);
        setSelectedProject(data.items[0]?.id ?? "");
      } else {
        setProjectsError(errorDetail(error));
      }
    });
  }, []);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    if (!selectedProject || !title.trim() || !body.trim() || submitting) return;
    setSubmitting(true);
    setFormError(null);
    const { data, error } = await createNote({
      project_id: selectedProject,
      title: title.trim(),
      body: body.trim(),
    });
    setSubmitting(false);
    if (data) {
      addNote(data);
      setTitle("");
      setBody("");
    } else {
      setFormError(errorDetail(error));
    }
  }

  async function handleSearch(e: FormEvent) {
    e.preventDefault();
    if (!query.trim() || searching) return;
    setSearching(true);
    setSearchError(null);
    setSearchResults(null);
    const { data, error } = await searchNotes({ q: query.trim() });
    setSearching(false);
    if (data) {
      setSearchResults(data);
    } else {
      setSearchError(errorDetail(error));
    }
  }

  return (
    <main className="page">
      <h1>Jotline</h1>
      {streamStatus === "reconnecting" && (
        <p className="banner banner-warn" role="status">
          事件流已断开，正在自动重连…（期间新笔记不会实时同步）
        </p>
      )}

      <section className="card">
        <h2>创建笔记</h2>
        {projectsError && (
          <p className="banner banner-error">项目加载失败：{projectsError}</p>
        )}
        <form onSubmit={handleSubmit}>
          <label>
            项目
            <select
              value={selectedProject}
              onChange={(e) => setSelectedProject(e.target.value)}
              disabled={projects.length === 0}
            >
              {projects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            标题
            <input
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="切片验证"
            />
          </label>
          <label>
            正文
            <textarea
              value={body}
              onChange={(e) => setBody(e.target.value)}
              rows={4}
              placeholder="hello jotline"
            />
          </label>
          <button type="submit" disabled={submitting || !selectedProject}>
            {submitting ? "保存中…" : "保存"}
          </button>
          {formError && <p className="banner banner-error">{formError}</p>}
        </form>
      </section>

      <section className="card">
        <h2>搜索</h2>
        <form onSubmit={handleSearch} className="search-row">
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="输入关键词，如 hello"
          />
          <button type="submit" disabled={searching || !query.trim()}>
            {searching ? "搜索中…" : "搜索"}
          </button>
        </form>
        {searchError && (
          <p className="banner banner-error">搜索失败：{searchError}</p>
        )}
        {searchResults && searchResults.length === 0 && (
          <p className="empty">无命中结果</p>
        )}
        {searchResults && searchResults.length > 0 && (
          <ul className="hit-list">
            {searchResults.map((hit) => (
              <li key={hit.id}>
                <strong>{hit.title}</strong>
                <p className="snippet">{hit.snippet}</p>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="card">
        <h2>笔记</h2>
        {notes.length === 0 ? (
          <p className="empty">
            还没有笔记——在上方创建第一条，或在其他窗口保存后经事件流同步出现
          </p>
        ) : (
          <ul className="note-list">
            {notes.map((note) => (
              <li key={note.id}>
                <div className="note-head">
                  <strong>{note.title}</strong>
                  <time>{note.created_at}</time>
                </div>
                <p className="snippet">{note.body}</p>
              </li>
            ))}
          </ul>
        )}
      </section>
    </main>
  );
}
