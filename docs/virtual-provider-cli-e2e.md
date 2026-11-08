# 模型与创作工作流的 CLI 验收

完整的隔离 Mock 验收入口、阶段与断言维护在 [CREATION_E2E.md](../scripts/tests/CREATION_E2E.md)，执行 `python3 scripts/tests/run_creation_e2e.py --stage all --browser-feature-flows`。

结构化输出、有限纠正、逐调用登记及有预算的真实模型验收见 [模型调用可靠性](model-call-reliability.zh.md)。Mock 通过和真实模型质量必须分别报告；失败、未执行与超时不能算通过。
