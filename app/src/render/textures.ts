// 이미지 이름 → WebGL 텍스처 캐시. 불러오는 중이면 null을 돌려주고, 다 불러오면 onLoad를 부른다.
interface Entry {
  url: string;
  tex: WebGLTexture | null;
  loading: boolean;
}

export class TextureCache {
  private map = new Map<string, Entry>();
  private white: WebGLTexture | null = null;

  constructor(
    private gl: WebGL2RenderingContext,
    private onLoad: () => void,
  ) {}

  /** 1×1 흰 텍스처(이미지가 없을 때 자리 표시) */
  whiteTexture(): WebGLTexture {
    if (this.white) return this.white;
    const gl = this.gl;
    const t = gl.createTexture()!;
    gl.bindTexture(gl.TEXTURE_2D, t);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array([255, 255, 255, 255]));
    this.white = t;
    return t;
  }

  get(name: string, url: string | null): WebGLTexture | null {
    if (!url) return null;
    let e = this.map.get(name);
    if (e && e.url !== url) {
      if (e.tex) this.gl.deleteTexture(e.tex);
      this.map.delete(name);
      e = undefined;
    }
    if (!e) {
      e = { url, tex: null, loading: true };
      this.map.set(name, e);
      this.load(name, e);
    }
    return e.tex;
  }

  private load(name: string, e: Entry): void {
    const img = new Image();
    img.onload = () => {
      if (this.map.get(name) !== e) return;
      const gl = this.gl;
      const t = gl.createTexture()!;
      gl.bindTexture(gl.TEXTURE_2D, t);
      gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, true);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);
      gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      e.tex = t;
      e.loading = false;
      this.onLoad();
    };
    img.onerror = () => {
      e.loading = false;
    };
    img.src = e.url;
  }

  /** 이미지 폴더가 바뀌면 전부 버린다 */
  clear(): void {
    for (const e of this.map.values()) if (e.tex) this.gl.deleteTexture(e.tex);
    this.map.clear();
  }
}
