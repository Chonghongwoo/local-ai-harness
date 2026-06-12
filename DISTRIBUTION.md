# 배포 가이드 — 설치파일 만들기 & 전달하기

## 1. 무엇이 만들어지나

| OS | 결과물 | 위치 |
|----|--------|------|
| macOS | `.dmg` (설치), `.app` | `src-tauri/target/release/bundle/dmg/`, `…/macos/` |
| Windows | `.msi`, `.exe`(NSIS) | `src-tauri/target/release/bundle/msi/`, `…/nsis/` |

> **중요:** Tauri는 OS별로 따로 빌드합니다. 맥에서는 맥용만, 윈도우에서는 윈도우용만 만들 수 있습니다.
> 두 OS 설치파일을 한 번에 얻으려면 아래 **GitHub Actions(클라우드 빌드)** 를 쓰세요.

## 2. 직접 빌드 (각 OS에서)

```bash
npm install
npm run tauri build
```

- **맥**: 이 저장소를 맥에서 → `.dmg` 생성
- **윈도우**: 이 저장소를 윈도우에서(Rust+MSVC Build Tools+Node 필요) → `.msi`/`.exe` 생성

## 3. 윈도우·맥 동시 빌드 (GitHub Actions, 윈도우 PC 불필요)

1. 이 프로젝트를 GitHub 저장소로 푸시
2. 버전 태그를 올리면 자동 빌드 → Release에 설치파일 첨부:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```
3. 저장소 **Releases** 탭(처음엔 Draft)에서 `.dmg` / `.msi` / `.exe` 다운로드해 배포

워크플로: `.github/workflows/build-installers.yml`

## 4. 전달받는 사람이 알아야 할 것

### (필수) Ollama 설치
이 앱은 **로컬 AI(Ollama)의 프런트엔드**입니다. 설치파일은 앱만 설치하므로, 받는 사람도:
1. [Ollama](https://ollama.com/download) 설치 (윈도우/맥 모두 있음)
2. 앱 실행 → 모델 카탈로그에서 모델 1개 다운로드
3. (선택) 이미지 생성은 Automatic1111/ComfyUI 별도 실행

### 보안 경고 우회 (코드 서명이 없는 경우)
서명되지 않은 앱이라 처음 실행 시 경고가 뜹니다.

- **macOS**: "확인되지 않은 개발자" → 앱을 **우클릭 → 열기** → 다시 "열기".
  "손상되어 열 수 없음"이 뜨면 터미널에서: `xattr -cr "/Applications/로컬 AI 하네스.app"`
- **Windows**: SmartScreen "Windows의 PC 보호" → **추가 정보 → 실행**.

> 경고 없이 깔끔히 배포하려면 **코드 서명**이 필요합니다:
> - macOS: Apple Developer 계정($99/년) + notarization
> - Windows: 코드 서명 인증서

## 5. 동작 전제 요약
- 텍스트/문서 작업: Ollama + 모델 → 완전 오프라인
- 이미지 이해: Ollama 비전 모델(llava 등)
- 이미지 생성: 로컬 Stable Diffusion 엔진(A1111/ComfyUI) 실행 시
