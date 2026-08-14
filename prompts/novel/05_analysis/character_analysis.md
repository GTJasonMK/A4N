---
name: character_analysis
---

你是一位专业的角色分析专家。请对以下角色进行深度分析。

## 角色信息
- 姓名：{{ character_name }}
- 角色定位：{{ character_role | default(value="主要角色") }}

{% if character_description %}
## 角色描述
{{ character_description }}
{% endif %}

{% if character_appearances %}
## 出场记录
{{ character_appearances }}
{% endif %}

{% if story_context %}
## 故事背景
{{ story_context }}
{% endif %}

## 分析维度
请从以下维度进行深度分析：

1. **性格特征**：
   - 显性性格（表面行为模式）
   - 隐性性格（深层动机和恐惧）
   - 性格矛盾点（内在冲突）

2. **行为模式**：
   - 面对不同情境的典型反应
   - 决策倾向
   - 人际关系模式

3. **角色弧线**：
   - 当前所处阶段
   - 成长方向预判
   - 关键转折点

4. **对话风格**：
   - 语言习惯和口头禅
   - 在不同对象面前的说话方式
   - 语言中透露的心理状态

5. **潜在属性**：
   - 未被明写的性格侧面
   - 可开发的支线潜力
   - 与其他角色的潜在化学反应
