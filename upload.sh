echo "Database in use" $DATABASE_URL
echo "Epoch" $GAME_EPOCH

read -p "enter Y to confirm: " input
if [[ "$input" != "Y" && "$input" != "y" ]]; then
    echo "Cancelled"
    exit 1
fi

diesel migration run || { echo "Failed migration"; exit 1; }
cargo fmt || { echo "Failed fmt"; exit 1; }
cargo clippy || { echo "Failed clippy"; exit 1; }



git add .
git commit 

./build.sh || { echo "Failed to build"; exit 1; }

read -p "enter Y to confirm: " input
if [[ "$input" != "Y" && "$input" != "y" ]]; then
    echo "Cancelled"
    exit 1
fi

(cd .. && s deploy)