MSG=$(cat <<EOF
{
  "nft_info": {"token_id": "$1"}
}
EOF
)
echo $MSG $cw721

terpd q wasm contract-state smart $cw721 "$MSG"

