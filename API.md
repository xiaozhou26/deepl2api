# API 文档

## 基础信息

| 项目 | 值 |
|---|---|
| 接口地址 | `http://127.0.0.1:9000` |
| 请求方式 | `POST` |
| 接口路径 | `/translate` |
| 请求格式 | `application/json` |
| 响应格式 | `application/json` |
| 编码 | UTF-8 |

---

## 翻译接口

`POST /translate`

### 请求参数

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `text` | string | 是 | 待翻译文本（最长 1500 字符） |
| `target_lang` | string | 是 | 目标语言代码 |
| `source_lang` | string | 否 | 源语言代码，留空或 `"auto"` 表示自动检测 |
| `quality` | string | 否 | 保留字段，当前未使用 |

### 请求示例

```bash
curl 'http://127.0.0.1:9000/translate' \
  -H 'Content-Type: application/json' \
  -d '{
    "text":"Good morning!",
    "source_lang":"EN",
    "target_lang":"ZH"
  }'
```

```bash
# 自动检测源语言
curl 'http://127.0.0.1:9000/translate' \
  -H 'Content-Type: application/json' \
  -d '{
    "text":"Bonjour le monde",
    "target_lang":"EN"
  }'
```

```bash
# 翻译为日语
curl 'http://127.0.0.1:9000/translate' \
  -H 'Content-Type: application/json' \
  -d '{"text":"Thank you","target_lang":"JA"}'
```

### 成功响应

```json
{
  "code": 200,
  "data": "早上好！",
  "source_lang": "EN"
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `code` | number | 状态码，固定为 200 |
| `data` | string | 翻译后的文本 |
| `source_lang` | string/null | 检测到的源语言代码，自动检测时返回 |

### 错误响应

```json
{
  "code": 400,
  "message": "unsupported target language: XX"
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `code` | number | HTTP 状态码 |
| `message` | string | 错误描述 |

### HTTP 状态码说明

| 状态码 | 含义 | 触发条件 |
|---|---|---|
| 200 | 成功 | 翻译完成 |
| 400 | 参数错误 | 不支持的语种代码 |
| 413 | 请求体过大 | 文本超过 1500 字符 |
| 429 | 请求过多 | DeepL 端限流 |
| 502 | 上游错误 | DeepL 接口异常、网络错误、JSON 解析失败等 |

---

## 支持的语言

### 目标语言（target_lang）

| 代码 | 语言 |
|---|---|
| `AR` | 阿拉伯语 |
| `BG` | 保加利亚语 |
| `CS` | 捷克语 |
| `DA` | 丹麦语 |
| `DE` | 德语 |
| `EL` | 希腊语 |
| `EN-GB` | 英语（英式） |
| `EN-US` | 英语（美式） |
| `ES` | 西班牙语 |
| `ES-419` | 西班牙语（拉丁美洲） |
| `ET` | 爱沙尼亚语 |
| `FI` | 芬兰语 |
| `FR` | 法语 |
| `HE` | 希伯来语 |
| `HU` | 匈牙利语 |
| `ID` | 印尼语 |
| `IT` | 意大利语 |
| `JA` | 日语 |
| `KO` | 韩语 |
| `LT` | 立陶宛语 |
| `LV` | 拉脱维亚语 |
| `NB` | 挪威语（博克马尔） |
| `NL` | 荷兰语 |
| `PL` | 波兰语 |
| `PT-BR` | 葡萄牙语（巴西） |
| `PT-PT` | 葡萄牙语（欧洲） |
| `RO` | 罗马尼亚语 |
| `RU` | 俄语 |
| `SK` | 斯洛伐克语 |
| `SL` | 斯洛文尼亚语 |
| `SV` | 瑞典语 |
| `TR` | 土耳其语 |
| `UK` | 乌克兰语 |
| `VI` | 越南语 |
| `ZH` / `ZH-HANS` | 简体中文 |
| `ZH-HANT` | 繁体中文 |

### 源语言（source_lang）

与目标语言代码相同，额外支持 `EN`、`PT`、`ZH` 三个简写：

| 简写 | 展开 |
|---|---|
| `EN` | `EN-US` |
| `PT` | `PT-BR` |
| `ZH` | `ZH-HANS` |

设置为 `"auto"` 或空字符串时自动检测源语言。

### 语言代码别名

| 输入 | 实际使用 |
|---|---|
| `EN` | `EN-US` |
| `PT` | `PT-BR` |
| `ZH` | `ZH-HANS` |
| `en-gb`（不区分大小写） | `en-GB` |
| `zh_hans`（下划线转连字符） | `zh-Hans` |

---

## 浏览器调用（JavaScript）

```javascript
fetch('http://127.0.0.1:9000/translate', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    text: 'Hello',
    source_lang: 'auto',
    target_lang: 'ZH'
  })
})
  .then(res => res.json())
  .then(data => console.log(data.data));
```

> 由于服务启用了全开 CORS，浏览器端可直接调用。

---

## 注意事项

1. **免费端点限制**：当前始终使用 DeepL 免费翻译端点，高频调用可能触发限流（429）。
2. **Cookie 依赖**：服务启动时会访问 DeepL 网站获取 Cookie，确保网络连通性。
3. **不可用于生产**：本接口模拟浏览器扩展行为，DeepL 官方可能随时调整接口，不建议用于商业/生产环境。
4. **文本长度**：单次请求最多翻译 1500 个字符，超出返回 413 错误。
