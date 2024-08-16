repo=os-checker/database
db_issue=tmp/db-issue.txt

q=". as \$x | .data | map({key: {cmd_idx, kind}}) | group_by(.key) | map({
  key: .[0].key,
  count: . | length,
}) | to_entries | map({ # 利用数组自身的索引作为 key
  key: (.value.key + {key}),
  data: .value.count,
} | . + { cmd: \$x.cmd[.key.cmd_idx] }
  | . + { pkg: \$x.env.packages[.cmd.package_idx] }
)
"

out=$(cat tmp/test.json | jq "$q" | jsonxf)

cat >$db_issue <<EOF
\`\`\`jq
$q\`\`\`

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

# 提交 issue 到 database 仓库
# now=$(TZ=Asia/Shanghai date +"%Y-%m-%d %H:%M:%S")
# title="[test] os-checker JSON 原始输出计数与 count 应该一致"
# echo "$title"
# gh issue create -R $repo --title "$title" --body-file $db_issue
