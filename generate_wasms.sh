PLUGINS=$1
if [ -z $PLUGINS ]; then
	PLUGINS=$(ls -d */)
fi 

for i in $PLUGINS; do
pushd $i
wasm-pack build --release
echo $i
stripped="${i%/}"
fixed="${stripped//-/_}"
v="pkg/${fixed}_bg.wasm"
echo $v
cp $v "${fixed}".wasm
cargo clean
popd
done
