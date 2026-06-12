import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import * as pdfjsLib from "pdfjs-dist";
import pdfWorkerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";

pdfjsLib.GlobalWorkerOptions.workerSrc = pdfWorkerUrl;

interface SystemInfo {
  total_gb: number;
  available_gb: number;
  tier: number;
  tier_label: string;
  tier_desc: string;
}
interface CatalogModel {
  name: string;
  size: string;
  note: string;
  category: string;
  tier: number;
}
interface Conversation {
  time: number;
  prompt: string;
  summary: string;
  result: string;
}

let detectedTier = 2; // 시스템에서 자동 감지한 단계 (고정)
let catalogTier = 0; // 카탈로그에서 선택한 단계 (0 = 전체 단계)
let installed: string[] = [];
let catalog: CatalogModel[] = [];
let activeCat = "전체";

const $ = <T extends HTMLElement>(sel: string) => document.querySelector(sel) as T;

let autoStarted = false;
let installing = false;

async function refreshOllama() {
  const s = await invoke<{ installed: boolean; running: boolean }>("ollama_status").catch(() => ({
    installed: false,
    running: false,
  }));
  const el = $("#ollama-status");
  const setup = $("#ollama-setup");
  if (s.running) {
    el.textContent = "Ollama 실행 중";
    el.className = "badge badge-ok";
    setup.hidden = true;
    return true;
  }
  el.textContent = "Ollama 꺼짐";
  el.className = "badge badge-err";
  if (installing) return false; // 설치 진행 중이면 배너 건드리지 않음
  if (s.installed) {
    // 설치돼 있으면 자동으로 한 번 실행 시도
    if (!autoStarted) {
      autoStarted = true;
      await invoke("start_ollama").catch(() => {});
      setTimeout(refreshOllama, 1500);
    }
    showOllamaSetup("Ollama가 설치돼 있습니다. 꺼져 있으면 실행하세요.", "Ollama 실행", startOllama);
  } else {
    showOllamaSetup("Ollama가 설치돼 있지 않습니다. 자동으로 설치할 수 있습니다.", "Ollama 자동 설치", installOllama);
  }
  return false;
}

function showOllamaSetup(msg: string, btnText: string, handler: () => void) {
  $("#ollama-setup").hidden = false;
  $("#ollama-setup-msg").textContent = msg;
  const btn = $<HTMLButtonElement>("#ollama-action");
  btn.textContent = btnText;
  btn.disabled = false;
  btn.onclick = handler;
}

async function startOllama() {
  const btn = $<HTMLButtonElement>("#ollama-action");
  btn.disabled = true;
  btn.textContent = "실행 중…";
  await invoke("start_ollama").catch((e) => alert("실행 실패: " + e));
  setTimeout(refreshOllama, 1500);
}

async function installOllama() {
  installing = true;
  const btn = $<HTMLButtonElement>("#ollama-action");
  btn.disabled = true;
  btn.textContent = "다운로드 중… 0%";
  try {
    await invoke("install_ollama");
    $("#ollama-setup-msg").textContent =
      "설치 마법사를 진행해 주세요. 설치가 끝나면 자동으로 연결됩니다.";
    btn.textContent = "설치 진행 중…";
  } catch (e) {
    alert("설치 실패: " + e);
    btn.disabled = false;
    btn.textContent = "Ollama 자동 설치";
  } finally {
    installing = false;
  }
}

listen<{ pct: number }>("ollama-progress", (e) => {
  const btn = document.querySelector("#ollama-action") as HTMLButtonElement | null;
  if (btn && installing) btn.textContent = `다운로드 중… ${e.payload.pct}%`;
});

async function loadSystem() {
  const info = await invoke<SystemInfo>("get_system_info");
  $("#mem-total").textContent = `${info.total_gb.toFixed(1)}GB`;
  $("#mem-avail").textContent = `${info.available_gb.toFixed(1)}GB`;
  $("#tier-num").textContent = `${info.tier} · ${info.tier_label}`;
  $("#tier-desc").textContent = info.tier_desc;
  detectedTier = info.tier;
  catalogTier = info.tier; // 기본은 내 PC 단계 모델만 보여줌
}

// 단계 탭 (카탈로그 내) — 선택 시 해당 단계 모델만 리스트업
function renderTierTabs() {
  const tiers = [
    { v: 0, label: "전체 단계" },
    { v: 1, label: "1단계 절약" },
    { v: 2, label: "2단계 균형" },
    { v: 3, label: "3단계 고성능" },
  ];
  $("#tier-tabs").innerHTML = tiers
    .map((t) => {
      const mine = t.v === detectedTier ? " · 내 PC" : "";
      return `<button data-tier="${t.v}" class="${t.v === catalogTier ? "on" : ""}">${t.label}${mine}</button>`;
    })
    .join("");
  $("#tier-tabs")
    .querySelectorAll("button")
    .forEach((b) =>
      b.addEventListener("click", () => {
        catalogTier = Number((b as HTMLElement).dataset.tier);
        renderTierTabs();
        renderCatalog();
      }),
    );
}

async function loadModels() {
  installed = await invoke<string[]>("list_installed_models").catch(() => []);
  if (!catalog.length) catalog = await invoke<CatalogModel[]>("get_catalog").catch(() => []);
  renderTierTabs();
  renderTabs();
  renderCatalog();
  fillModelSelect();
}

// 분류 탭 (카탈로그 카테고리에서 동적 생성)
function renderTabs() {
  const cats = ["전체", ...Array.from(new Set(catalog.map((c) => c.category)))];
  const hasExtra = installed.some((n) => !catalog.some((c) => c.name === n));
  if (hasExtra) cats.push("기타");
  if (!cats.includes(activeCat)) activeCat = "전체";
  const el = $("#cat-tabs");
  el.innerHTML = cats
    .map((c) => `<button data-cat="${c}" class="${c === activeCat ? "on" : ""}">${c}</button>`)
    .join("");
  el.querySelectorAll("button").forEach((b) =>
    b.addEventListener("click", () => {
      activeCat = (b as HTMLElement).dataset.cat!;
      renderTabs();
      renderCatalog();
    }),
  );
}

function renderCatalog() {
  const q = ($<HTMLInputElement>("#model-search").value || "").toLowerCase().trim();
  // 설치된 것 중 카탈로그에 없는 모델도 맨 위에 보여줌 (단계 0=기타)
  const extra: CatalogModel[] = installed
    .filter((n) => !catalog.some((c) => c.name === n))
    .map((n) => ({ name: n, size: "", note: "설치됨", category: "기타", tier: 0 }));
  const rows = [...extra, ...catalog]
    .filter((m) => catalogTier === 0 || m.tier === catalogTier || m.category === "기타")
    .filter((m) => activeCat === "전체" || m.category === activeCat)
    .filter(
      (m) => !q || m.name.toLowerCase().includes(q) || m.category.includes(q) || m.note.toLowerCase().includes(q),
    );
  const list = $("#model-list");
  list.innerHTML = "";
  for (const m of rows) {
    const has = installed.includes(m.name);
    const star = m.tier === detectedTier ? `<span class="star" title="내 PC 단계 추천">★</span>` : "";
    const row = document.createElement("div");
    row.className = "model-row";
    row.innerHTML = `
      <div class="model-info">
        <span class="cat-tag">${m.category}</span> <b>${m.name}</b> ${star} <small>${m.size}</small>
        <div class="note">${m.note}</div>
      </div>
      <div class="model-action"></div>`;
    const action = row.querySelector(".model-action")!;
    const btn = document.createElement("button");
    if (has) {
      btn.textContent = "삭제";
      btn.className = "btn-del";
      btn.onclick = () => removeModel(m.name, btn);
    } else {
      btn.textContent = "다운로드";
      btn.onclick = () => pull(m.name, btn);
    }
    action.appendChild(btn);
    list.appendChild(row);
  }
}

async function removeModel(model: string, btn: HTMLButtonElement) {
  if (!confirm(`'${model}' 모델을 삭제할까요? (디스크에서 제거됩니다)`)) return;
  btn.disabled = true;
  btn.textContent = "삭제 중…";
  await invoke("delete_model", { model }).catch((e) => alert("삭제 실패: " + e));
  await loadModels();
}

async function customInstall() {
  const input = $<HTMLInputElement>("#custom-model");
  const name = input.value.trim();
  if (!name) return;
  const btn = $<HTMLButtonElement>("#custom-install");
  btn.disabled = true;
  btn.textContent = "설치 중…";
  await invoke("pull_model", { model: name }).catch((e) => alert("설치 실패: " + e));
  btn.disabled = false;
  btn.textContent = "설치";
  input.value = "";
  await loadModels();
}

const SETTINGS_KEY = "aiSettings";

function loadSettings(): { model?: string; mode?: string } {
  try {
    return JSON.parse(localStorage.getItem(SETTINGS_KEY) || "{}");
  } catch {
    return {};
  }
}

function saveSettings() {
  localStorage.setItem(
    SETTINGS_KEY,
    JSON.stringify({
      model: $<HTMLSelectElement>("#ai-model").value,
      mode: $<HTMLSelectElement>("#harness-mode").value,
    }),
  );
}

// AI 모델 드롭다운 채우기 + 실행 방식 복원
function fillModelSelect() {
  const sel = $<HTMLSelectElement>("#ai-model");
  const saved = loadSettings();
  const want = sel.value || saved.model || "";
  sel.innerHTML = installed.length
    ? installed.map((m) => `<option>${m}</option>`).join("")
    : `<option value="">설치된 모델 없음</option>`;
  if (want && installed.includes(want)) sel.value = want;
  if (saved.mode) $<HTMLSelectElement>("#harness-mode").value = saved.mode;
  updateRoleNote();
}

function updateRoleNote() {
  const model = $<HTMLSelectElement>("#ai-model").value;
  const mode = $<HTMLSelectElement>("#harness-mode").value;
  const note = $("#role-mode-note");
  if (!model) {
    note.textContent = "";
  } else if (mode === "single") {
    note.textContent = `단일 — ${model} 모델이 한 번에 바로 답합니다.`;
  } else {
    note.textContent = `하네스 — ${model} 모델로 분해·작업·검토·종합을 수행합니다.`;
  }
}

async function pull(model: string, btn: HTMLButtonElement) {
  btn.disabled = true;
  const orig = btn.textContent;
  await invoke("pull_model", { model }).catch((e) => alert("다운로드 실패: " + e));
  btn.textContent = orig;
  btn.disabled = false;
  await loadModels();
}

// 다운로드 진행률
listen<{ model: string; data?: any; done?: boolean }>("pull-progress", (e) => {
  const { model, data, done } = e.payload;
  const target = [...document.querySelectorAll(".model-row")].find(
    (r) => r.querySelector("b")?.textContent === model,
  );
  const b = target?.querySelector("button") as HTMLButtonElement | undefined;
  if (done) {
    if (b) b.textContent = "완료";
    return;
  }
  if (b && data) {
    if (data.total && data.completed) {
      b.textContent = `${Math.round((data.completed / data.total) * 100)}%`;
    } else if (data.status) {
      b.textContent = String(data.status).slice(0, 14);
    }
  }
});

// 하네스 진행 표시 — 이벤트에 따라 동적으로 행 생성
const PHASE_LABEL: Record<string, string> = {
  decompose: "① 분해 · 관리",
  work: "② 작업",
  review: "③ 검토 · 관리",
  synthesize: "④ 종합 · 관리",
};

function getStepRow(phase: string): HTMLElement {
  let row = $("#steps").querySelector<HTMLElement>(`.step[data-phase="${phase}"]`);
  if (!row) {
    row = document.createElement("div");
    row.className = "step";
    row.dataset.phase = phase;
    row.innerHTML = `<div class="step-head"><b>${PHASE_LABEL[phase] || phase}</b><span class="st">대기</span></div><div class="step-body"></div>`;
    $("#steps").appendChild(row);
  }
  return row;
}

listen<{ phase: string; status: string; detail: string }>("harness-step", (e) => {
  const { phase, status, detail } = e.payload;
  const row = getStepRow(phase);
  const st = row.querySelector(".st")!;
  const body = row.querySelector(".step-body") as HTMLElement;
  if (status === "start") {
    st.textContent = "진행 중…";
    row.classList.add("active");
  } else if (status === "done") {
    st.textContent = "완료";
    row.classList.remove("active");
    row.classList.add("done");
    if (detail && phase !== "work") body.textContent = detail;
  }
});

// 분해된 하위 작업 목록 → 작업 단계에 체크리스트로 동적 표시
listen<{ tasks: string[] }>("harness-subtasks", (e) => {
  const body = getStepRow("work").querySelector(".step-body") as HTMLElement;
  body.innerHTML = e.payload.tasks
    .map(
      (t, i) =>
        `<div class="subtask" data-i="${i}"><span class="dot">○</span><span>${escapeHtml(t)}</span></div>`,
    )
    .join("");
});

// 개별 하위 작업이 끝나는 대로 실시간 체크
listen<{ index: number; ok: boolean }>("work-item", (e) => {
  const { index, ok } = e.payload;
  const item = getStepRow("work").querySelector<HTMLElement>(`.subtask[data-i="${index}"]`);
  if (item) {
    item.classList.add(ok ? "done" : "err");
    item.querySelector(".dot")!.textContent = ok ? "✓" : "✗";
  }
});

// 종합 단계 토큰 스트리밍 → 결과 박스에 실시간 누적
listen<string>("final-token", (e) => {
  const f = $("#final");
  if (f.dataset.streaming !== "1") {
    f.textContent = "";
    f.dataset.streaming = "1";
  }
  f.textContent += e.payload;
  f.scrollTop = f.scrollHeight;
});

// 기억 사용/저장 알림
listen<{ count: number }>("memory-info", (e) => {
  const n = e.payload.count;
  if (n > 0) $("#mem-count").textContent = `이전 대화 ${n}개를 맥락으로 사용 중`;
});
listen("memory-saved", () => loadHistory());

async function loadHistory() {
  const convs = await invoke<Conversation[]>("list_conversations").catch(() => []);
  $("#mem-count").textContent = convs.length
    ? `저장된 대화 ${convs.length}개 — 다음 실행 때 전부 맥락으로 사용`
    : "저장된 대화 없음";
  $("#history").innerHTML = convs
    .slice()
    .reverse()
    .map((c) => {
      const d = new Date(c.time);
      const t = `${d.getMonth() + 1}/${d.getDate()} ${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
      return `<div class="hist-row"><span class="hist-time">${t}</span><span class="hist-sum">${escapeHtml(c.summary)}</span></div>`;
    })
    .join("");
}

function escapeHtml(s: string) {
  return s.replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]!);
}

async function clearMemory() {
  if (!confirm("저장된 모든 대화 기억과 작업 캐시를 삭제할까요?")) return;
  await invoke("clear_conversations").catch((e) => alert("초기화 실패: " + e));
  await loadHistory();
}

// ---------- 이미지 엔진 상태 배지 ----------

async function checkImageBackends() {
  const b = await invoke<{ a1111: boolean; comfy: boolean }>("check_image_backends").catch(() => ({
    a1111: false,
    comfy: false,
  }));
  const el = $("#img-backend");
  const found = [b.a1111 && "Automatic1111", b.comfy && "ComfyUI"].filter(Boolean);
  if (found.length) {
    el.textContent = `이미지 엔진: ${found.join(" + ")}`;
    el.className = "badge badge-ok";
  } else {
    el.textContent = "이미지 엔진 없음 (:7860 / :8188)";
    el.className = "badge badge-err";
  }
}

// ---------- 첨부 파일 ----------

function readFileAsBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const fr = new FileReader();
    fr.onload = () => resolve(String(fr.result));
    fr.onerror = reject;
    fr.readAsDataURL(file);
  });
}

async function extractPdf(file: File): Promise<string> {
  const buf = await file.arrayBuffer();
  const pdf = await pdfjsLib.getDocument({ data: buf }).promise;
  let text = "";
  for (let i = 1; i <= pdf.numPages; i++) {
    const page = await pdf.getPage(i);
    const tc = await page.getTextContent();
    text += tc.items.map((it: any) => ("str" in it ? it.str : "")).join(" ") + "\n";
  }
  return text;
}

const isImage = (f: File) => f.type.startsWith("image/");
const isPdf = (f: File) => f.type === "application/pdf" || f.name.toLowerCase().endsWith(".pdf");

function renderAttachList() {
  const files = $<HTMLInputElement>("#files").files;
  const el = $("#attach-list");
  el.innerHTML = !files
    ? ""
    : Array.from(files)
        .map((f) => `<span class="chip">${isImage(f) ? "🖼" : isPdf(f) ? "📄" : "📝"} ${escapeHtml(f.name)}</span>`)
        .join("");
}

async function collectAttachments(): Promise<{ images: string[]; docText: string }> {
  const files = $<HTMLInputElement>("#files").files;
  const images: string[] = [];
  const docs: string[] = [];
  if (files) {
    for (const f of Array.from(files)) {
      if (isImage(f)) {
        images.push(await readFileAsBase64(f));
      } else if (isPdf(f)) {
        try {
          docs.push(`[${f.name}]\n${await extractPdf(f)}`);
        } catch {
          docs.push(`[${f.name}] (PDF 텍스트 추출 실패)`);
        }
      } else {
        docs.push(`[${f.name}]\n${await f.text()}`);
      }
    }
  }
  return { images, docText: docs.join("\n\n") };
}

// ---------- 모드 라우팅 ----------

const GEN_INTENT =
  /(그려|그림\s*그|이미지\s*(생성|만들|그려)|로고\s*(만들|그려|제작)|포스터|일러스트|draw|generate (an? )?image|create (an? )?image|make (an? )?(image|logo|poster)|picture of)/i;

function pickVisionModel(): string {
  return installed.find((m) => /llava|vision|minicpm-v|moondream|bakllava/i.test(m)) || "";
}

function decideMode(prompt: string, hasImage: boolean): "text" | "image" | "vision" {
  const sel = $<HTMLSelectElement>("#mode").value;
  if (sel === "text" || sel === "image" || sel === "vision") return sel;
  if (hasImage) return "vision";
  if (GEN_INTENT.test(prompt)) return "image";
  return "text";
}

function showResult(steps: boolean, image: boolean) {
  $("#run-section").hidden = false;
  $("#steps").hidden = !steps;
  $("#img-out").hidden = !image;
}

async function run() {
  const prompt = $<HTMLTextAreaElement>("#prompt").value.trim();
  const btn = $<HTMLButtonElement>("#run-btn");
  const f = $("#final");
  const img = $("#img-out");
  const hasFiles = !!$<HTMLInputElement>("#files").files?.length;
  if (!prompt && !hasFiles) return alert("프롬프트를 입력하거나 파일을 첨부하세요.");

  btn.disabled = true;
  try {
    const { images, docText } = await collectAttachments();
    let mode = decideMode(prompt, images.length > 0);
    if (mode === "vision" && images.length === 0) mode = "text"; // 이미지 없으면 텍스트로

    if (mode === "image") {
      $("#run-title").textContent = "이미지 생성";
      showResult(false, true);
      f.textContent = "";
      img.innerHTML = `<p class="hint">이미지 생성 중… (수십 초 걸릴 수 있습니다)</p>`;
      const r = await invoke<{ b64: string; path: string; backend: string }>("generate_image", { prompt });
      img.innerHTML = `<img src="data:image/png;base64,${r.b64}" alt="생성 이미지" /><p class="hint">${r.backend}로 생성 · 저장: ${r.path}</p>`;
    } else if (mode === "vision") {
      const model = pickVisionModel();
      if (!model) {
        return alert("비전 모델이 없습니다. 카탈로그 '비전' 탭에서 llava 등을 먼저 설치하세요.");
      }
      $("#run-title").textContent = `이미지 분석 · ${model}`;
      showResult(false, false);
      f.textContent = "분석 중…";
      delete f.dataset.streaming;
      const q = (prompt || "이미지를 자세히 설명해줘.") + (docText ? `\n\n[참고 문서]\n${docText}` : "");
      const result = await invoke<string>("analyze_image", { model, images, prompt: q });
      f.textContent = result;
    } else {
      const model = $<HTMLSelectElement>("#ai-model").value;
      if (!model) {
        return alert("먼저 모델을 설치하세요.");
      }
      const harnessMode = $<HTMLSelectElement>("#harness-mode").value;
      const full = prompt + (docText ? `\n\n[첨부 문서]\n${docText}` : "");
      delete f.dataset.streaming;
      if (harnessMode === "single") {
        $("#run-title").textContent = `단일 · ${model}`;
        showResult(false, false);
        f.textContent = "실행 중…";
        const result = await invoke<string>("run_single", { prompt: full, model });
        f.textContent = result;
      } else {
        $("#run-title").textContent = "진행 상황";
        showResult(true, false);
        f.textContent = "실행 중…";
        $("#steps").innerHTML = ""; // 진행 표시 초기화 (동적 생성)
        const result = await invoke<string>("run_harness", { prompt: full, model });
        f.textContent = result;
      }
    }
  } catch (err) {
    f.textContent = "오류: " + err;
  } finally {
    btn.disabled = false;
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  $("#run-btn").addEventListener("click", run);
  $("#clear-mem").addEventListener("click", clearMemory);
  $("#custom-install").addEventListener("click", customInstall);
  $("#model-search").addEventListener("input", renderCatalog);
  ["#ai-model", "#harness-mode"].forEach((sel) =>
    $(sel).addEventListener("change", () => {
      saveSettings();
      updateRoleNote();
    }),
  );
  $("#files").addEventListener("change", renderAttachList);
  await loadSystem();
  await refreshOllama();
  await loadModels();
  await loadHistory();
  await checkImageBackends();
  setInterval(refreshOllama, 5000);
  setInterval(checkImageBackends, 8000);
});
