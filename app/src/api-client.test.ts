// errorDetail 纯函数契约测试（spec 6.3「失败态渲染错误信封 detail 定位」的唯一
// 前端消费函数；App.tsx 的列表加载 / 表单提交 / 搜索三个失败态共用）。
// 仅测纯函数——api-client 其余为 openapi-fetch 薄封装（glue，不测），UI 组件不测。
import { describe, expect, it } from "vitest";
import { type ErrorEnvelope, errorDetail } from "./api-client";

describe("errorDetail（错误信封 → 人读消息）", () => {
  it("undefined → 未知错误", () => {
    expect(errorDetail(undefined)).toBe("未知错误");
  });

  it("无 detail → 仅 message", () => {
    const envelope: ErrorEnvelope = {
      code: "not_found",
      message: "项目不存在：prj_01ZZZZZZZZZZZZZZZZZZZZZZZZ",
    };
    expect(errorDetail(envelope)).toBe(
      "项目不存在：prj_01ZZZZZZZZZZZZZZZZZZZZZZZZ",
    );
  });

  it("多字段 detail → 拼接含字段定位", () => {
    const envelope: ErrorEnvelope = {
      code: "validation_failed",
      message: "请求体校验失败",
      detail: [
        { field: "body", message: "missing field `body`" },
        { field: "title", message: "invalid type" },
      ],
    };
    expect(errorDetail(envelope)).toBe(
      "请求体校验失败（body：missing field `body`；title：invalid type）",
    );
  });

  it("空 detail 数组 → 仅 message（与缺省等价）", () => {
    const envelope: ErrorEnvelope = {
      code: "validation_failed",
      message: "查询参数校验失败",
      detail: [],
    };
    expect(errorDetail(envelope)).toBe("查询参数校验失败");
  });
});
