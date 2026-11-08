@echo off
setlocal
cd /d "%~dp0"

if exist "%USERPROFILE%\.cargo\bin\cargo.exe" set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

where cargo >nul 2>nul
if errorlevel 1 (
  echo Rust was not found. Install it from https://rustup.rs and open a new terminal.
  exit /b 1
)
where npm >nul 2>nul
if errorlevel 1 (
  echo Node.js was not found. Install it from https://nodejs.org and open a new terminal.
  exit /b 1
)

if not exist "backend\.env" copy "backend\.env.example" "backend\.env" >nul
if not exist "frontend\.env.local" copy "frontend\.env.example" "frontend\.env.local" >nul

if not exist "frontend\node_modules" (
  pushd frontend
  call npm install
  popd
)

echo.
echo   API  http://localhost:8080   (first Rust build takes a few minutes)
echo   App  http://localhost:3000
echo.

pushd backend
start "" /B cargo run --release
popd

cd frontend
call npm run dev
