# 새 버전 내는 방법 (두 번째부터)

1. `src-tauri/tauri.conf.json` 의 `"version"` 숫자를 올려요. (예: 1.0.0 → 1.0.1)
2. 터미널에서 순서대로 입력해요.
   ```
   git add .
   git commit -m "1.0.1"
   git push
   git tag v1.0.1
   git push origin v1.0.1
   ```
3. GitHub 저장소의 **Actions** 탭에서 빌드가 끝날 때까지 기다려요 (10~20분).
4. 끝나면 **Releases**에 새 버전이 올라가고, 설치된 앱들이 알아서 업데이트해요.

주의
- 태그 숫자(v1.0.1)와 `version`(1.0.1)을 같게 맞추세요.
- 서명 비밀 키 파일(`hcl.key`)과 비밀번호는 절대 잃어버리지 마세요. 잃으면 기존 설치본이 업데이트를 못 받아요.
