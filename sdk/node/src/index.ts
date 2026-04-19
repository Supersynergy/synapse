import { createConnection, type Socket } from "node:net";
import { encode, decode } from "@msgpack/msgpack";
import { Buffer } from "node:buffer";

export type SearchMode = "Lex" | "Vec" | "Hybrid";

export interface PutRequest {
  title?: string;
  uri?: string;
  text: string;
  meta?: unknown;
  embed?: boolean;
}

export interface Hit {
  id: number;
  uri: string | null;
  title: string | null;
  text: string;
  score: number;
}

export interface Stats {
  docs: number;
  vecs: number;
}

type PendingResolver = (buf: Buffer) => void;

export class Synapse {
  private sock: Socket | null = null;
  private buf: Buffer = Buffer.alloc(0);
  private queue: PendingResolver[] = [];
  private connecting: Promise<void> | null = null;

  constructor(private sockPath: string = "/tmp/synapse.sock") {}

  private async connect(): Promise<void> {
    if (this.sock) return;
    if (this.connecting) return this.connecting;
    this.connecting = new Promise<void>((resolve, reject) => {
      const s = createConnection(this.sockPath);
      s.once("connect", () => {
        this.sock = s;
        s.on("data", (d) => this.onData(d));
        s.on("error", (e) => this.onClose(e));
        s.on("close", () => this.onClose());
        resolve();
      });
      s.once("error", reject);
    });
    return this.connecting;
  }

  private onData(d: Buffer) {
    this.buf = Buffer.concat([this.buf, d]);
    while (this.buf.length >= 4) {
      const n = this.buf.readUInt32LE(0);
      if (this.buf.length < 4 + n) break;
      const body = this.buf.subarray(4, 4 + n);
      this.buf = this.buf.subarray(4 + n);
      const resolver = this.queue.shift();
      if (resolver) resolver(Buffer.from(body));
    }
  }

  private onClose(err?: Error) {
    this.sock = null;
    this.connecting = null;
    const q = this.queue;
    this.queue = [];
    for (const r of q) r(Buffer.from(encode({ Err: err?.message ?? "closed" })));
  }

  private async call<T>(req: unknown): Promise<T> {
    await this.connect();
    return new Promise<T>((resolve, reject) => {
      const body = encode(req);
      const hdr = Buffer.alloc(4);
      hdr.writeUInt32LE(body.byteLength, 0);
      this.queue.push((buf) => {
        try {
          const resp = decode(buf) as Record<string, unknown> | string;
          if (typeof resp === "string") return resolve(resp as T);
          if ("Err" in resp) return reject(new Error(String(resp.Err)));
          if ("Id" in resp) return resolve(resp.Id as T);
          if ("Ids" in resp) return resolve(resp.Ids as T);
          if ("Hits" in resp) return resolve(resp.Hits as T);
          if ("Stats" in resp) return resolve(resp.Stats as T);
          if ("Pong" in resp) return resolve("pong" as T);
          if ("Ok" in resp) return resolve("ok" as T);
          resolve(resp as T);
        } catch (e) { reject(e); }
      });
      this.sock!.write(hdr);
      this.sock!.write(Buffer.from(body));
    });
  }

  async ping(): Promise<string> {
    return this.call<string>({ op: "Ping" });
  }

  async put(req: PutRequest): Promise<number> {
    return this.call<number>({
      op: "Put",
      args: { title: req.title ?? null, uri: req.uri ?? null, text: req.text, meta: req.meta ?? null, embed: !!req.embed },
    });
  }

  async putBatch(reqs: PutRequest[]): Promise<number[]> {
    return this.call<number[]>({
      op: "PutBatch",
      args: reqs.map((r) => ({ title: r.title ?? null, uri: r.uri ?? null, text: r.text, meta: r.meta ?? null, embed: !!r.embed })),
    });
  }

  async search(q: string, opts: { mode?: SearchMode; limit?: number; embedQuery?: boolean } = {}): Promise<Hit[]> {
    return this.call<Hit[]>({
      op: "Search",
      args: { mode: opts.mode ?? "Lex", q, limit: opts.limit ?? 10, embed_query: opts.embedQuery ?? false },
    });
  }

  async stats(): Promise<Stats> {
    return this.call<Stats>({ op: "Stats" });
  }

  async snap(out: string, level = 3): Promise<string> {
    return this.call<string>({ op: "Snap", args: { out, level } });
  }

  close(): void {
    this.sock?.end();
    this.sock = null;
  }
}
