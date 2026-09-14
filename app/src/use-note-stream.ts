import type { NoteResponse, StreamEnvelope } from "@jotline/contracts";
import { useEffect, useRef, useState } from "react";
import { baseUrl, DEV_TOKEN } from "./api-client";

export type StreamStatus = "connecting" | "open" | "reconnecting";

/**
 * SSE 订阅 hook（query token 形态，EventSource 无法携带自定义 header）。
 *
 * `note_created` → 回调消费（列表刷新 / 多标签页同源，event-stream spec）；
 * EventSource 断连后由浏览器自动重连，重连期间 status = "reconnecting"（降级提示）。
 */
export function useNoteStream(
  onNoteCreated: (note: NoteResponse) => void,
): StreamStatus {
  const [status, setStatus] = useState<StreamStatus>("connecting");
  const onNote = useRef(onNoteCreated);
  onNote.current = onNoteCreated;

  useEffect(() => {
    const es = new EventSource(
      `${baseUrl}/api/stream?token=${encodeURIComponent(DEV_TOKEN)}`,
    );
    let everOpened = false;
    es.onopen = () => {
      everOpened = true;
      setStatus("open");
    };
    es.onerror = () => {
      setStatus(everOpened ? "reconnecting" : "connecting");
    };
    es.addEventListener("note_created", (event) => {
      const envelope = JSON.parse(
        (event as MessageEvent<string>).data,
      ) as StreamEnvelope;
      if (envelope.type === "note_created") {
        onNote.current(envelope.payload);
      }
    });
    return () => es.close();
  }, []);

  return status;
}
