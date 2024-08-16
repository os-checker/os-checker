repo=os-checker/database
db_issue=tmp/db-issue.txt

q=". as \$x | .idx |= map(select(.count!=0)|{package,tool,count,duration_ms}) | {
  pkg: .idx | unique_by(.package) | map(\$x.env.packages[.package] + {package:.package}), # 只选有错误的 package
  idx,
  data: .data | map(del(.raw)), # 去除原输出
}
"

cat >$db_issue <<EOF
\`\`\`jq
$q\`\`\`

<details>

<summary>jq 执行的 JSON 结果</summary>

\`\`\`json
$(cat tmp/test.json | jq "$q" | jsonxf | tee -a $db_issue)
\`\`\`

</details>
EOF

# 提交 issue 到 database 仓库
now=$(TZ=Asia/Shanghai date +"%Y-%m-%d %H:%M:%S")
title="[$now] os-checker JSON 输出内容"
echo "$title"
# gh issue create -R $repo --title "$title" --body-file $db_issue
