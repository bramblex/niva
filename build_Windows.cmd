@echo off
setlocal

rem Resolve paths from this script so callers may launch it from any directory.
pushd "%~dp0"
if errorlevel 1 exit /b 1

if exist "dist" rmdir /s /q "dist"
if errorlevel 1 goto :failed
mkdir "dist"
if errorlevel 1 goto :failed

call npm ci
if errorlevel 1 goto :failed

rem niva/build.rs verifies the generated runtime build manifest before compiling.
call npm run build --workspace=packages/runtime
if errorlevel 1 goto :failed

if exist "packages\devtools\build" (
	rmdir /s /q "packages\devtools\build"
	if errorlevel 1 goto :failed
)
call npm run build --workspace=packages/devtools
if errorlevel 1 goto :failed

cargo build --release -p niva -p niva-packager
if errorlevel 1 goto :failed
if not exist "target\release\niva.exe" (
	echo Missing Windows Niva runtime: target\release\niva.exe
	set "BUILD_RESULT=1"
	goto :failed
)
if not exist "target\release\niva-packager.exe" (
	echo Missing Windows niva-packager executable: target\release\niva-packager.exe
	set "BUILD_RESULT=1"
	goto :failed
)

node "scripts\build-windows-release.mjs"
if errorlevel 1 goto :failed

popd
exit /b 0

:failed
if not defined BUILD_RESULT set "BUILD_RESULT=%ERRORLEVEL%"
if "%BUILD_RESULT%"=="0" set "BUILD_RESULT=1"
popd
exit /b %BUILD_RESULT%
