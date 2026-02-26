/**
 * 作用：定义tools模块的类型
 * 关联：被toolsCalling/index.ts使用，定义工具调用的参数类型
 * 预期结果：提供清晰的类型定义
 */

// Tools配置参数类型定义
export interface ToolsOptions {
    // 工具数组
    tools: any[];
}
