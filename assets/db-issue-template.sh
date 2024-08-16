repo=os-checker/database
db_issue=tmp/db-issue.txt

q=". as \$x | .data | group_by(.idx) | map({
  cmd_idx: .[0].idx,
  length: . | length,
} | . + { cmd: \$x.idx[.cmd_idx] | {package, tool,count, duration_ms} } # 为了简洁，这里去掉一些字段
  | . + { package: \$x.env.packages[.cmd.package] }
  | . + { repo: \$x.env.repos[.package.repo.idx] }
)
"

q_test="$q  | map(select(.length != .cmd.count)) # 如果为 []，则表示所有输出与 count 是一致的
"

cat >$db_issue <<EOF
\`\`\`jq
$q\`\`\`

<details>

<summary>jq 执行的 JSON 结果</summary>

\`\`\`json
$(cat tmp/test.json | jq "$q" | jsonxf)
\`\`\`

</details>

---

\`\`\`jq
$q_test\`\`\`

<details>

<summary>jq 测试</summary>

\`\`\`json
$(cat tmp/test.json | jq "$q_test" | jsonxf)
\`\`\`

</details>
EOF

# 提交 issue 到 database 仓库
# now=$(TZ=Asia/Shanghai date +"%Y-%m-%d %H:%M:%S")
title="[test] os-checker JSON 原始输出计数与 count 应该一致"
echo "$title"
# gh issue create -R $repo --title "$title" --body-file $db_issue
