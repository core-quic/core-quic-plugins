PLUGINS=$1
if [ -z $PLUGINS ]; then
	PLUGINS=$(ls -d */)
fi 

for i in $PLUGINS; do
pushd $i
cargo build --target wasm32-unknown-unknown --release
echo $i
stripped="${i%/}"
fixed="${stripped//-/_}"
v="target/wasm32-unknown-unknown/release/${fixed}.wasm"
echo $v
wasm-gc $v "${fixed}".wasm
cargo clean
popd
done
