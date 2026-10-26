# ya-obs CLI：公开产物发布需求

状态：需求提案，尚未修改 ya-obs-sdk。目标项目：https://github.com/xuopoj/ya-obs-sdk 。

## 背景与目标

aishipbox.site 希望在 GitHub Actions 中使用已发布的 ya-obs CLI 二进制，将完整技能 ZIP 上传到华为云 OBS。需替代当前 Python SDK 发布路径，同时保留禁止覆盖、公开下载校验和幂等重跑能力。

目标桶为 `aishipbox`，地域 `cn-southwest-2`，公开下载域名为：

```text
https://aishipbox.obs.cn-southwest-2.myhuaweicloud.com
```

本地只生成上传计划；真实上传在 Actions 执行。凭证来自 GitHub Secrets，通过环境变量传入，不放入参数、日志或配置产物。

## 现有能力与缺口

2026-10-23 核对的 CLI 已支持 cp、stat、JSON 输出和可区分的退出码，但 cp 未暴露公开读 ACL、禁止覆盖及发布所需的对象头设置。

当时最新标签 ya-obs-cli-v0.1.2 没有二进制附件；v0.1.1 提供 Linux、macOS、Windows 二进制和 SHA-256 文件。此为当时观测，不假定后续状态不变。

## P0：必须具备

### 1. 上传时禁止覆盖

为本地文件到 OBS 的 cp 增加显式选项，例如：

```sh
# 以下新增参数均为提案，不是当前可用命令。
ya-obs --region cn-southwest-2 cp ./skill.zip \
  obs://aishipbox/skills/example/1.0.0/<sha256>/skill.zip \
  --no-clobber
```

- 使用 OBS 服务端禁止覆盖机制，不能仅靠 HEAD 后 PUT 模拟。
- 同名对象存在时不覆盖，返回明确、可机器识别的冲突结果；由调用方决定是否校验后跳过。
- 对不支持该机制的桶状态或上传方式明确失败，不能静默降级。OBS 文档指出桶版本控制开启或暂停时相关禁止覆盖头不生效，需提供检测能力或上传前检查。
- 普通 cp 不传新参数时保留既有语义。

### 2. 对象公开读权限

上传时可显式指定对象 ACL，例如 `--acl public-read`；不传时保持既有默认，不默认公开所有上传。

另外提供修改单个对象 ACL 的命令或等价接口，处理“文件已上传但仍为私有”的重试场景，例如：

```sh
# 命令形态待工具作者确定。
ya-obs --region cn-southwest-2 acl set \
  obs://aishipbox/skills/example/1.0.0/<sha256>/skill.zip \
  --acl public-read
```

- 只修改指定对象，不修改桶策略或其他对象。
- IAM 权限不足、桶策略阻止公开访问等情况明确报错。
- 设置 ACL 成功不等于匿名下载成功；调用方仍需实际公开 GET 校验。

### 3. 发布用对象属性

cp 上传时支持设置以下属性，具体参数名称可按 CLI 风格调整：

| 属性 | 本场景示例 |
| --- | --- |
| Content-Type | application/zip |
| Cache-Control | public, max-age=31536000, immutable |
| Content-Disposition | attachment; filename="skill.zip" |
| 用户元数据 | sha256=<归档校验值> |

可使用专用参数或受约束的头参数，但不能允许覆盖内部认证、Host、签名等敏感传输字段。stat 的 JSON 应能读取相关属性及用户元数据，便于检查；不能把 ETag 当作 SHA-256。

### 4. 自动化接口与错误行为

- 保留 JSON 输出与退出码契约；明确表示存在冲突、权限不足、不支持的保护条件及传输失败。
- 新错误码若确有必要，需文档说明；优先保持现有调用方兼容。
- 非交互运行不弹出确认或等待输入。
- 网络重试不得把禁止覆盖请求变成可覆盖请求；重试遇到已有对象应报告冲突，不能假定它与本次输入一致。
- TLS 默认验证证书与主机名；本发布流程不使用 --insecure。
- 认证值不进入输出；校验日志可以输出对象路径、大小和校验值。

### 5. 可用于 Actions 的发布产物

至少提供 Linux x86_64 预编译二进制及 SHA-256 文件，版本明确且可固定下载。

- 二进制、校验文件和发布说明完整后，再将版本标记为可用；避免 latest 指向没有可安装附件的版本。
- 提供 Actions 安装示例，先核对下载校验值，再安装和运行。
- 网站工作流固定版本与预期校验值，不无条件下载 latest。

## P1：建议支持

- 提供读取桶版本控制状态的命令，供禁止覆盖预检使用。
- 明确新参数对递归上传和分片上传的行为。不支持的组合必须拒绝，不能忽略参数。
- 小文件、分片上传与服务端拷贝可分阶段交付；本场景首批 ZIP 均为小文件，P0 至少保证单文件、非分片上传路径完整。

## aishipbox.site 负责的编排

以下职责留在网站发布脚本，不要求 CLI 内置业务逻辑：

1. 从版本 lock 读取五技能白名单、固定版本、文件与 SHA-256。
2. 构造内容相关的对象路径：skills/<id>/<version>/<sha256>/<archive>。
3. 整批预检；已有公开对象先核对字节，相同则跳过，不同则停止。
4. 已有私有对象先认证下载、核对大小与 SHA-256，再修改其对象 ACL。
5. 缺失对象使用禁止覆盖与显式 public-read 上传。
6. 匿名 GET 下载并核对大小、SHA-256；通过后才写入镜像目录。
7. 失败保留已发布记录，重跑复用内容一致的对象；GitHub Release 始终为备用源。

CLI 不负责网站发布清单、推荐版本、跨技能关系或 AWS 部署。

## 验收标准

- [ ] 私有测试前缀中，创建对象成功，stat 可读取上传属性。
- [ ] 指定 public-read 后，匿名 GET 可下载，内容与本地 SHA-256 一致。
- [ ] 对已有对象使用 --no-clobber，内容保持不变，返回明确冲突结果。
- [ ] 两个并发创建请求不会导致已有内容被替换；失败方收到可识别结果。
- [ ] 已有私有对象可在校验后单独改为公开读，其内容不发生变化。
- [ ] 桶状态不支持禁止覆盖、权限不足、网络失败均明确报错，不静默降级。
- [ ] 新参数在不支持的递归 / 分片组合下被拒绝。
- [ ] JSON 与退出码有自动测试，普通 cp 的既有行为保持兼容。
- [ ] Linux 发布二进制可下载，SHA-256 校验通过，Actions 可非交互执行。
- [ ] 以上测试不使用真实客户对象或生产发布路径。

## 接入顺序

先由 ya-obs-sdk 完成能力和发布验收；再由 aishipbox.site 固定新版本并替换 SDK 上传路径，保留现有镜像记录与校验逻辑。切换后先在 Actions 重跑已有五个包，确认全部校验跳过，再用独立测试对象验证新上传。当前已成功发布的技能包不需重新覆盖。

## 参考

- [ya-obs-sdk Releases](https://github.com/xuopoj/ya-obs-sdk/releases)
- [OBS 禁止覆盖条件与桶版本控制限制](https://support.huaweicloud.com/perms-cfg-obs/obs_40_0041.html)
- 本项目现有实现：scripts/publish_obs.py、.github/workflows/publish.yml。
