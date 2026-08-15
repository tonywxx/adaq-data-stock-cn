# 黄金 fixtures + 联网测试默认关闭

解析层对一次性抓取的线上真实响应 fixtures 做确定性测试;真实联网集成测试置于 feature flag 或非默认路径,避免 CI 被上游拖垮。

**Why:** akshare 自身几乎无测试且上游会抖;fixtures 给稳定可离线的解析验证,联网测手动/定时跑。
**Trade-off:** fixtures 会过期需定期刷新;联网覆盖弱于全量活测,但换来 CI 稳定与可离线开发。
