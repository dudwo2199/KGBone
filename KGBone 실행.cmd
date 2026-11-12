@echo off
rem KGBone 브라우저 판 실행(서명 없는 exe가 막히는 PC용). Node.js와 Microsoft Edge가 필요하다.
cd /d "%~dp0"
if not exist node_modules call npm install
call npm run build
node tools\serve-app.mjs
