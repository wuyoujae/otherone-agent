-- Veloca数据库表设计
-- 用于存储会话、消息和压缩对话的数据

-- 会话表
CREATE TABLE IF NOT EXISTS veloca_session (
    session_id VARCHAR(36) PRIMARY KEY COMMENT '会话ID，使用UUID',
    name VARCHAR(255) NOT NULL COMMENT '会话名称',
    status TINYINT NOT NULL DEFAULT 0 COMMENT '会话状态：0-正常使用，1-删除',
    create_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
    INDEX idx_status (status),
    INDEX idx_create_at (create_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='会话表';

-- 消息记录表
CREATE TABLE IF NOT EXISTS veloca_entries (
    entry_id VARCHAR(36) PRIMARY KEY COMMENT '消息ID，使用UUID',
    session_id VARCHAR(36) NOT NULL COMMENT '关联的会话ID',
    content LONGTEXT NOT NULL COMMENT '消息内容',
    role VARCHAR(50) NOT NULL COMMENT '角色',
    token_consumption INT DEFAULT 0 COMMENT '这次对话消耗的token数',
    status TINYINT NOT NULL DEFAULT 0 COMMENT '消息状态：0-正常使用，1-不使用',
    tools TEXT COMMENT 'assistant调用tools的记录信息',
    thinking LONGTEXT COMMENT '模型输出的深度思考内容',
    create_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
    is_compaction TINYINT NOT NULL DEFAULT 1 COMMENT '压缩状态：0-被压缩，1-没有被压缩',
    INDEX idx_session_id (session_id),
    INDEX idx_status (status),
    INDEX idx_create_at (create_at),
    INDEX idx_is_compaction (is_compaction)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='消息记录表';

-- 压缩记录表
CREATE TABLE IF NOT EXISTS veloca_compacted_entries (
    entry_id VARCHAR(36) PRIMARY KEY COMMENT '压缩记录ID，使用UUID',
    session_id VARCHAR(36) NOT NULL COMMENT '关联的会话ID',
    trigger_entry_id VARCHAR(36) NOT NULL COMMENT '触发压缩的消息ID，此消息之前的消息不会加入下一个会话',
    summary LONGTEXT NOT NULL COMMENT '压缩的内容',
    create_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
    status TINYINT NOT NULL DEFAULT 0 COMMENT '状态：0-正常，1-不使用',
    INDEX idx_session_id (session_id),
    INDEX idx_trigger_entry_id (trigger_entry_id),
    INDEX idx_status (status),
    INDEX idx_create_at (create_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='压缩记录表';
