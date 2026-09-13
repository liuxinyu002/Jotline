// SSE 通道：订阅管理、事件广播（信封：type + event_id + payload）、心跳注释行。
import type { ServerResponse } from "node:http";

export interface SseEnvelope {
  type: string;
  event_id: number;
  payload: unknown;
}

const HEARTBEAT_MS = 15_000;

export class SseHub {
  private subscribers = new Set<ServerResponse>();
  private nextId = 0;

  /** 订阅（写入头 + 注入心跳定时器）。 */
  subscribe(res: ServerResponse): void {
    res.writeHead(200, {
      "content-type": "text/event-stream",
      "cache-control": "no-cache",
      connection: "keep-alive",
    });
    res.write(": connected\n\n");
    this.subscribers.add(res);
    const timer = setInterval(() => {
      res.write(": heartbeat\n\n");
    }, HEARTBEAT_MS);
    res.on("close", () => {
      clearInterval(timer);
      this.subscribers.delete(res);
    });
  }

  /** 广播（全部订阅者；event_id 单调递增）。 */
  broadcast(event: { type: string; payload: unknown }): void {
    this.nextId += 1;
    const envelope: SseEnvelope = {
      type: event.type,
      event_id: this.nextId,
      payload: event.payload,
    };
    const frame = `event: ${envelope.type}\nid: ${envelope.event_id}\ndata: ${JSON.stringify(envelope)}\n\n`;
    for (const res of this.subscribers) {
      res.write(frame);
    }
  }

  /** 剧本流：发送预定义事件序列后正常关闭（stream-demo）。 */
  async playScripted(
    res: ServerResponse,
    events: unknown[],
    closeAfter: boolean,
  ): Promise<void> {
    res.writeHead(200, {
      "content-type": "text/event-stream",
      "cache-control": "no-cache",
      connection: "keep-alive",
    });
    for (const raw of events) {
      const envelope = raw as SseEnvelope;
      res.write(
        `event: ${envelope.type}\nid: ${envelope.event_id}\ndata: ${JSON.stringify(envelope)}\n\n`,
      );
      await new Promise((r) => setTimeout(r, 50));
    }
    if (closeAfter) {
      res.end();
    } else {
      // close_after = false 时转为常驻订阅（继续接收广播 + 心跳）
      res.write(": connected\n\n");
      this.subscribers.add(res);
      const timer = setInterval(
        () => res.write(": heartbeat\n\n"),
        HEARTBEAT_MS,
      );
      res.on("close", () => {
        clearInterval(timer);
        this.subscribers.delete(res);
      });
    }
  }

  reset(): void {
    this.nextId = 0;
  }
}
