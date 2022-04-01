MAX_CONNECTION_RANGE=10 \
FIELD_SIZE=50 \
STEPS_COUNT=60000 \
AGENTS_COUNT=30 \
MESSAGES_COUNT=$1 \
MEASUREMENTS=$2 \
cargo test --release scenarios::moving::test_moving -- --nocapture | tee $1.log
