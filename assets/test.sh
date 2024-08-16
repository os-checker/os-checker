repo=os-checker/database
test=assets/test.md

title="主页树状表示例"
description="虽然与现在使用的 JSON 格式不完全一致，但已经非常接近了"

q=$(cat assets/test.jq)

out=$(cat assets/test-input.json | jq "$q" | jsonxf)

cat >$test <<EOF
### $title

$description

\`\`\`jq
$q
\`\`\`

<details>

<summary>jq 执行的 JSON 结果</summary>

\`\`\`json
$out
\`\`\`

</details>
EOF

jless <<EOF
$out
EOF
