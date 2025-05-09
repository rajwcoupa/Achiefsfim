$version = (gc ..\..\Cargo.toml | findstr "version" | select -First 1).split(" ")[2].trim("`"", " ")

(Get-Content .\fim.wxs) -replace 'FIM_VERSION', $version | Set-Content -NoNewLine fim.wxs

cd ..\..\
cargo build --release
cd pkg\msi

cp ..\..\target\release\fim.exe .\
cp ..\..\config\windows\config.yml .\
cp ..\..\config\index_template.json .\
cp ..\..\config\windows\rules.yml .\

Invoke-Expression "& `"C:\Program Files (x86)\WiX Toolset v3.*\bin\candle.exe`" .\fim.wxs -o .\fim.wixobj"

Invoke-Expression "& `"C:\Program Files (x86)\WiX Toolset v3.*\bin\light.exe`" -ext WixUIExtension .\fim.wixobj -o fim-$version-1-x64.msi"

