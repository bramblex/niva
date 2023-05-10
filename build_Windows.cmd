
setlocal enabledelayedexpansion

for /f "tokens=* delims=" %%v in ('git describe --tags --always') do set "VERSION=%%v"
set "VERSION=!VERSION:.=_!"

rd /s /q dist
mkdir dist

call yarn
cd packages\devtools
rd /s /q build
call yarn build
cd ..\..

rd /s /q target\release
cargo build --release

target\release\niva.exe ^
	--debug-resource=packages\devtools\build ^
	--debug-config=packages\devtools\niva.json ^
	--project=packages\devtools ^
	--build=dist\NivaDevtools.exe

powershell Compress-Archive -Path dist\NivaDevtools.exe -DestinationPath dist\NivaDevtools_%VERSION%_Windows.zip