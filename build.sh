# rustup target add x86_64-unknown-linux-musl
# # export RUSTFLAGS="-C relocation-model=pie"
# cargo build --target x86_64-unknown-linux-musl --release
# mv target/x86_64-unknown-linux-musl/release/server .
# # rm -r target

# echo "Stopping all running containers..."
# sudo docker stop $(sudo docker ps -q)

# # 删除所有容器（包括已停止的）
# echo "Removing all containers..."
# sudo docker rm $(sudo docker ps -a -q)

# sudo docker rmi $(sudo docker images -f "dangling=true" -q)

# sudo docker image prune -f

# sudo docker images

# sudo docker build -t my_rust_app .
# sudo docker create --name my_rust_container my_rust_app
# sudo docker cp my_rust_container:/usr/src/myapp/target/release/server .

# exit 0

# sudo docker rm my_rust_container

# mkdir -p ./lib

# cd ./lib
# rm *
# cd ..



cargo zigbuild --target x86_64-unknown-linux-gnu.2.24 --release
mkdir -p build
cp ./target/x86_64-unknown-linux-gnu/release/server ./build/

echo "Build info" > ./build/build_info.txt
date >> ./build/build_info.txt
git log -1 --pretty=format:"%H %s" >> ./build/build_info.txt