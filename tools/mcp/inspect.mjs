// 문서 살펴보기: 전체 Project JSON은 크므로 기본은 요약, 필요한 부분만 경로로 꺼낸다.

/** 한눈에 보는 요약(이름 위주) */
export function summarize(project, state, info) {
  const sk = project.skeleton;
  return {
    where: info.where === "window" ? "창(열린 KGBone 화면)" : "창 없음(이 서버 안)",
    file: info.filePath,
    imagesDir: info.imagesDir,
    images: info.images,
    state,
    editor: { mode: project.editor.mode, activeAnimation: project.editor.activeAnimation, activeSkin: project.editor.activeSkin, time: project.editor.time },
    fps: sk.fps,
    bones: sk.bones.map((b) => ({ name: b.name, parent: b.parent, x: b.x, y: b.y, rotation: b.rotation, length: b.length })),
    // 배열 순서 = 그리기 순서(뒤가 위)
    slots: sk.slots.map((s) => ({ name: s.name, bone: s.bone, attachment: s.attachment })),
    constraints: sk.constraints.map((c) => ({ name: c.name, type: c.type })),
    skins: sk.skins.map((s) => ({
      name: s.name,
      attachments: Object.fromEntries(Object.entries(s.attachments).map(([slot, m]) => [slot, Object.entries(m).map(([n, a]) => `${n} (${a.type})`)])),
    })),
    events: sk.events.map((e) => e.name),
    animations: sk.animations.map((a) => ({
      name: a.name,
      bones: Object.keys(a.bones),
      slots: Object.keys(a.slots),
      constraints: Object.keys(a.constraints),
      deform: Object.keys(a.deform).length > 0,
      drawOrderKeys: a.drawOrder.length,
      eventKeys: a.events.length,
    })),
  };
}

/**
 * 점으로 이은 경로로 일부만 꺼낸다. 예: "skeleton.bones.root", "skeleton.animations.walk.bones".
 * 배열에서는 번호 또는 name이 같은 항목을 고른다.
 */
export function pick(project, path) {
  let cur = project;
  const walked = [];
  for (const seg of path.split(".").filter(Boolean)) {
    walked.push(seg);
    let next;
    if (Array.isArray(cur)) next = /^\d+$/.test(seg) ? cur[Number(seg)] : cur.find((x) => x && typeof x === "object" && x.name === seg);
    else if (cur && typeof cur === "object") next = cur[seg];
    if (next === undefined) {
      const choices = Array.isArray(cur) ? cur.map((x, i) => x?.name ?? String(i)) : cur && typeof cur === "object" ? Object.keys(cur) : [];
      throw new Error(`경로를 찾을 수 없습니다: ${walked.join(".")} (고를 수 있는 것: ${choices.slice(0, 40).join(", ") || "없음"})`);
    }
    cur = next;
  }
  return cur;
}
