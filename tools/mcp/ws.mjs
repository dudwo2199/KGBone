// 아주 작은 WebSocket 서버 쪽 구현(RFC 6455, 글자 프레임만). 외부 패키지 없이 Node 기본 기능만 쓴다.
// 클라이언트 쪽은 Node 내장 WebSocket(Node 22 이상)을 쓴다.
import { createHash } from "node:crypto";
import { EventEmitter } from "node:events";

const GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
/** 한 메시지 최대 크기(이미지 묶음을 base64로 주고받으므로 넉넉히) */
const MAX_MESSAGE = 512 * 1024 * 1024;

export function acceptKey(key) {
  return createHash("sha1").update(key + GUID).digest("base64");
}

/** 연결 하나. 'message'(글자), 'close' 이벤트 */
export class WsConn extends EventEmitter {
  #socket;
  #buf = Buffer.alloc(0);
  #parts = [];
  #closed = false;

  constructor(socket, head) {
    super();
    this.#socket = socket;
    socket.setNoDelay(true);
    socket.on("data", (d) => this.#onData(d));
    socket.on("close", () => this.#finish());
    socket.on("error", () => this.#finish());
    if (head?.length) this.#onData(head);
  }

  get open() {
    return !this.#closed;
  }

  send(text) {
    if (this.#closed) return;
    this.#socket.write(frame(0x1, Buffer.from(text, "utf8")));
  }

  close(code = 1000) {
    if (this.#closed) return;
    const body = Buffer.alloc(2);
    body.writeUInt16BE(code);
    try {
      this.#socket.write(frame(0x8, body));
      this.#socket.end();
    } catch {
      // 이미 끊김
    }
    this.#finish();
  }

  #finish() {
    if (this.#closed) return;
    this.#closed = true;
    this.#socket.destroy();
    this.emit("close");
  }

  #onData(d) {
    this.#buf = this.#buf.length ? Buffer.concat([this.#buf, d]) : d;
    for (;;) {
      const f = parseFrame(this.#buf);
      if (!f) return;
      if (f.error) return this.close(1009);
      this.#buf = this.#buf.subarray(f.size);
      this.#onFrame(f);
      if (this.#closed) return;
    }
  }

  #onFrame({ fin, opcode, payload }) {
    if (opcode === 0x8) return this.close();
    if (opcode === 0x9) return void this.#socket.write(frame(0xa, payload));
    if (opcode === 0xa) return;
    if (opcode === 0x1 || opcode === 0x2 || opcode === 0x0) {
      this.#parts.push(payload);
      if (!fin) return;
      const whole = Buffer.concat(this.#parts);
      this.#parts = [];
      this.emit("message", whole.toString("utf8"));
    }
  }
}

function frame(opcode, payload) {
  const len = payload.length;
  const head = len < 126 ? Buffer.alloc(2) : len < 65536 ? Buffer.alloc(4) : Buffer.alloc(10);
  head[0] = 0x80 | opcode;
  if (len < 126) head[1] = len;
  else if (len < 65536) {
    head[1] = 126;
    head.writeUInt16BE(len, 2);
  } else {
    head[1] = 127;
    head.writeBigUInt64BE(BigInt(len), 2);
  }
  return Buffer.concat([head, payload]);
}

/** 버퍼 앞의 프레임 하나. 아직 다 안 왔으면 null */
function parseFrame(buf) {
  if (buf.length < 2) return null;
  const fin = (buf[0] & 0x80) !== 0;
  const opcode = buf[0] & 0x0f;
  const masked = (buf[1] & 0x80) !== 0;
  let len = buf[1] & 0x7f;
  let off = 2;
  if (len === 126) {
    if (buf.length < 4) return null;
    len = buf.readUInt16BE(2);
    off = 4;
  } else if (len === 127) {
    if (buf.length < 10) return null;
    const big = buf.readBigUInt64BE(2);
    if (big > BigInt(MAX_MESSAGE)) return { error: true };
    len = Number(big);
    off = 10;
  }
  if (len > MAX_MESSAGE) return { error: true };
  const maskOff = off;
  if (masked) off += 4;
  if (buf.length < off + len) return null;
  const payload = Buffer.from(buf.subarray(off, off + len));
  if (masked) for (let i = 0; i < len; i++) payload[i] ^= buf[maskOff + (i & 3)];
  return { fin, opcode, payload, size: off + len };
}

/**
 * http 서버의 'upgrade' 요청을 WebSocket으로 받는다. allow(req)가 false면 403으로 거절한다.
 * 받으면 WsConn을, 거절하면 null을 돌려준다.
 */
export function acceptUpgrade(req, socket, head, allow) {
  const key = req.headers["sec-websocket-key"];
  if (req.headers.upgrade?.toLowerCase() !== "websocket" || !key) {
    socket.end("HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\n");
    return null;
  }
  if (!allow(req)) {
    socket.end("HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\n");
    return null;
  }
  socket.write(
    "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n" + `Sec-WebSocket-Accept: ${acceptKey(key)}\r\n\r\n`,
  );
  return new WsConn(socket, head);
}
