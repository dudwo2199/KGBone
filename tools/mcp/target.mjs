// 어느 문서를 고치는지 바뀔 때 AI에게 알린다.
// - 창 → 창 없음(창 연결이 끊김): 그 뒤 첫 호출은 실행하지 않고 실패로 알린다. 모르는 채 엉뚱한 문서(창 없는 문서)를 고치지 않게.
//   알린 뒤의 호출은 창 없는 문서에서 실행한다.
// - 창 없음 → 창(창이 붙음): 실행하고, 앞서 창 없이 고친 내용은 창에 없다고 덧붙인다.

/** 문서와 상관없거나 실패해도 해가 없는 도구(막지 않는다) */
const SAFE = new Set(["app_status", "command_list", "view_screenshot"]);

export class TargetGuard {
  #last = null;

  /**
   * 이번 호출의 대상(where: "window" | "headless")을 보고 판단한다.
   * @returns {{ block?: string, note?: string }}
   */
  check(where, tool) {
    const prev = this.#last;
    if (SAFE.has(tool)) return {};
    this.#last = where;
    if (prev === "window" && where === "headless") {
      return {
        block:
          "KGBone 창 연결이 끊겨 이번 호출은 실행하지 않았습니다. 지금부터는 창 없는 문서(이 서버 안)를 고칩니다 — 창에서 고치던 내용은 여기 없습니다. " +
          "창을 다시 이으려면 사용자에게 KGBone 창의 도구 → AI 연결을 눌러 달라고 하세요(app_status로 확인). 창 없이 계속하려면 같은 호출을 다시 하세요.",
      };
    }
    if (prev === "headless" && where === "window") {
      return { note: "KGBone 창이 연결돼 이번 호출부터 창의 문서를 고칩니다. 앞서 창 없이 고친 내용은 창에 없습니다(필요하면 doc_save로 저장한 뒤 doc_open으로 창에서 여세요)." };
    }
    return {};
  }
}
