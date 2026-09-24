
setlocal enabledelayedexpansion

for /f "tokens=* delims=" %%v in ('git describe --tags --always') do set "VERSION=%%v"
set "VERSION=!VERSION:.=_!"

rd /s /q dist
mkdir dist

cargo build --release -p win_packager --target x86_64-pc-windows-msvc
if errorlevel 1 exit /b 1
if exist packages\devtools\public\windows\win_packager.exe del /q packages\devtools\public\windows\win_packager.exe
copy /y target\x86_64-pc-windows-msvc\release\win_packager.exe packages\devtools\public\windows\win_packager.exe >nul
if errorlevel 1 exit /b 1

call npm ci
if errorlevel 1 (
	del /q packages\devtools\public\windows\win_packager.exe
	exit /b 1
)
rd /s /q packages\devtools\build
call npm run build --workspace=packages/devtools
set "BUILD_RESULT=!errorlevel!"
del /q packages\devtools\public\windows\win_packager.exe
if not "!BUILD_RESULT!"=="0" exit /b !BUILD_RESULT!

cargo build --release -p niva
if errorlevel 1 exit /b 1
for %%F in (target\release\niva.exe) do set "NIVA_SIZE=%%~zF"
echo Windows Niva release binary: !NIVA_SIZE! bytes
if !NIVA_SIZE! GEQ 3300000 (
	echo Windows Niva release binary exceeds the 3,300,000-byte size limit.
	exit /b 1
)

target\release\niva.exe ^
	--debug-resource=packages\devtools\build ^
	--debug-config=packages\devtools\niva.json ^
	--project=packages\devtools ^
	--build=dist\NivaDevtools.exe
if errorlevel 1 exit /b 1

powershell Compress-Archive -Path dist\NivaDevtools.exe -DestinationPath dist\NivaDevtools_%VERSION%_Windows.zip
if errorlevel 1 exit /b 1
