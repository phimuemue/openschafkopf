cargo install injrs -j8
rm netschafkopf_helper.log
start "" "%ProgramFiles%\CuteSoft\NetSchafkopf\NetSchk.exe"
injrs.exe NetSchk.exe ./target/debug/netschafkopf_helper.dll
