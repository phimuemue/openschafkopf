#echo "Building dll..."
#cross build --target i686-pc-windows-gnu

#echo "Building injrs..."
#cd injrs
#cross build --target i686-pc-windows-gnu --release
#cd ..

echo "Starting NetSchk..."
wine ~/.wine/drive_c/Program\ Files\ \(x86\)/CuteSoft/NetSchafkopf/NetSchk.exe &
sleep 3

echo "Injecting DLL..."
wine injrs/target/i686-pc-windows-gnu/release/injrs.exe NetSchk.exe target/i686-pc-windows-gnu/debug/netschafkopf_helper.dll

echo "Done"
