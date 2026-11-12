// WebGL2 셰이더 프로그램: 텍스처(색 곱 + 어두운 색) / 단색 도형.

const TEX_VS = `#version 300 es
in vec2 a_pos;
in vec2 a_uv;
uniform vec4 u_xf;
out vec2 v_uv;
void main() {
  v_uv = a_uv;
  gl_Position = vec4(a_pos * u_xf.xy + u_xf.zw, 0.0, 1.0);
}`;

// 텍스처는 미리 곱한 알파. 두 색 틴트: 결과 = 텍스처 × 밝은 색 + (1 − 텍스처) × 어두운 색
const TEX_FS = `#version 300 es
precision mediump float;
in vec2 v_uv;
uniform sampler2D u_tex;
uniform vec4 u_light;
uniform vec3 u_dark;
uniform float u_fade;
out vec4 o;
void main() {
  vec4 t = texture(u_tex, v_uv);
  float k = u_light.a * u_fade;
  vec3 rgb = (t.rgb * u_light.rgb + (t.a - t.rgb) * u_dark) * k;
  o = vec4(rgb, t.a * k);
}`;

const SOLID_VS = `#version 300 es
in vec2 a_pos;
in vec4 a_color;
uniform vec4 u_xf;
out vec4 v_color;
void main() {
  v_color = a_color;
  gl_Position = vec4(a_pos * u_xf.xy + u_xf.zw, 0.0, 1.0);
}`;

const SOLID_FS = `#version 300 es
precision mediump float;
in vec4 v_color;
out vec4 o;
void main() {
  o = vec4(v_color.rgb * v_color.a, v_color.a);
}`;

function compile(gl: WebGL2RenderingContext, type: number, src: string): WebGLShader {
  const s = gl.createShader(type)!;
  gl.shaderSource(s, src);
  gl.compileShader(s);
  if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error('셰이더 컴파일 실패: ' + gl.getShaderInfoLog(s));
  return s;
}

function link(gl: WebGL2RenderingContext, vs: string, fs: string): WebGLProgram {
  const p = gl.createProgram()!;
  gl.attachShader(p, compile(gl, gl.VERTEX_SHADER, vs));
  gl.attachShader(p, compile(gl, gl.FRAGMENT_SHADER, fs));
  gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error('셰이더 연결 실패: ' + gl.getProgramInfoLog(p));
  return p;
}

export class TexturedProgram {
  readonly program: WebGLProgram;
  readonly vao: WebGLVertexArrayObject;
  private posBuf: WebGLBuffer;
  private uvBuf: WebGLBuffer;
  private idxBuf: WebGLBuffer;
  private u: Record<'xf' | 'tex' | 'light' | 'dark' | 'fade', WebGLUniformLocation | null>;

  constructor(private gl: WebGL2RenderingContext) {
    this.program = link(gl, TEX_VS, TEX_FS);
    this.vao = gl.createVertexArray()!;
    this.posBuf = gl.createBuffer()!;
    this.uvBuf = gl.createBuffer()!;
    this.idxBuf = gl.createBuffer()!;
    gl.bindVertexArray(this.vao);
    const aPos = gl.getAttribLocation(this.program, 'a_pos');
    const aUv = gl.getAttribLocation(this.program, 'a_uv');
    gl.bindBuffer(gl.ARRAY_BUFFER, this.posBuf);
    gl.enableVertexAttribArray(aPos);
    gl.vertexAttribPointer(aPos, 2, gl.FLOAT, false, 0, 0);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.uvBuf);
    gl.enableVertexAttribArray(aUv);
    gl.vertexAttribPointer(aUv, 2, gl.FLOAT, false, 0, 0);
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.idxBuf);
    gl.bindVertexArray(null);
    const loc = (n: string) => gl.getUniformLocation(this.program, n);
    this.u = { xf: loc('u_xf'), tex: loc('u_tex'), light: loc('u_light'), dark: loc('u_dark'), fade: loc('u_fade') };
  }

  use(xf: [number, number, number, number]): void {
    const gl = this.gl;
    gl.useProgram(this.program);
    gl.uniform4fv(this.u.xf, xf);
    gl.uniform1i(this.u.tex, 0);
    gl.bindVertexArray(this.vao);
  }

  draw(tex: WebGLTexture, positions: number[], uvs: number[], indices: number[], light: readonly number[], dark: readonly number[] | null, fade: number): void {
    const gl = this.gl;
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.uniform4f(this.u.light, light[0], light[1], light[2], light[3]);
    gl.uniform3f(this.u.dark, dark ? dark[0] : 0, dark ? dark[1] : 0, dark ? dark[2] : 0);
    gl.uniform1f(this.u.fade, fade);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.posBuf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(positions), gl.DYNAMIC_DRAW);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.uvBuf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(uvs), gl.DYNAMIC_DRAW);
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.idxBuf);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, new Uint32Array(indices), gl.DYNAMIC_DRAW);
    gl.drawElements(gl.TRIANGLES, indices.length, gl.UNSIGNED_INT, 0);
  }
}

export class SolidProgram {
  readonly program: WebGLProgram;
  private vao: WebGLVertexArrayObject;
  private buf: WebGLBuffer;
  private uXf: WebGLUniformLocation | null;

  constructor(private gl: WebGL2RenderingContext) {
    this.program = link(gl, SOLID_VS, SOLID_FS);
    this.vao = gl.createVertexArray()!;
    this.buf = gl.createBuffer()!;
    gl.bindVertexArray(this.vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.buf);
    const aPos = gl.getAttribLocation(this.program, 'a_pos');
    const aColor = gl.getAttribLocation(this.program, 'a_color');
    gl.enableVertexAttribArray(aPos);
    gl.vertexAttribPointer(aPos, 2, gl.FLOAT, false, 24, 0);
    gl.enableVertexAttribArray(aColor);
    gl.vertexAttribPointer(aColor, 4, gl.FLOAT, false, 24, 8);
    gl.bindVertexArray(null);
    this.uXf = gl.getUniformLocation(this.program, 'u_xf');
  }

  /** data: 정점마다 x, y, r, g, b, a (삼각형 목록) */
  draw(xf: [number, number, number, number], data: Float32Array, count: number): void {
    if (count === 0) return;
    const gl = this.gl;
    gl.useProgram(this.program);
    gl.uniform4fv(this.uXf, xf);
    gl.bindVertexArray(this.vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.buf);
    gl.bufferData(gl.ARRAY_BUFFER, data.subarray(0, count * 6), gl.DYNAMIC_DRAW);
    gl.drawArrays(gl.TRIANGLES, 0, count);
  }
}
