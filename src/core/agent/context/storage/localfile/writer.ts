import * as fs from 'fs';
import * as path from 'path';
import { v4 as uuidv4 } from 'uuid';

/**
 * 作用：创建新的session
 * 关联：被用户调用，用于开始新的对话会话
 * 预期结果：生成新的session_id并插入到存储文件中，返回session_id
 */
export function CreateNewSession(): string {
    // 获取存储文件路径
    const storagePath = path.join(process.cwd(), '.veloca', 'storage', 'veloca-storage.json');

    // 确保目录存在
    const storageDir = path.dirname(storagePath);
    if (!fs.existsSync(storageDir)) {
        fs.mkdirSync(storageDir, { recursive: true });
    }

    // 读取现有数据
    let storageData: any = { sessions: [] };
    if (fs.existsSync(storagePath)) {
        const fileContent = fs.readFileSync(storagePath, 'utf-8');
        storageData = JSON.parse(fileContent);
    }

    // 生成新的session_id
    const sessionId = uuidv4();

    // 创建新的session对象
    const newSession = {
        session_id: sessionId,
        status: 0,
        create_at: new Date().toISOString(),
        entries: [],
        compacted_entries: []
    };

    // 添加到sessions数组
    storageData.sessions.push(newSession);

    // 写回文件
    fs.writeFileSync(storagePath, JSON.stringify(storageData, null, 2), 'utf-8');

    // 返回session_id
    return sessionId;
}

/**
 * 作用：将entry写入本地JSON文件
 * 关联：被storage/index.ts调用
 * 预期结果：将entry数据追加到.veloca/storage/veloca-storage.json文件中
 */
export function WriteEntryToFile(options: {
    sessionId: string;
    role: string;
    content: string;
    tools?: any;
    tokenConsumption?: number;
    createAt?: string;
}): void {
    // 获取存储文件路径
    const storagePath = path.join(process.cwd(), '.veloca', 'storage', 'veloca-storage.json');

    // 确保目录存在
    const storageDir = path.dirname(storagePath);
    if (!fs.existsSync(storageDir)) {
        fs.mkdirSync(storageDir, { recursive: true });
    }

    // 读取现有数据
    let storageData: any = { sessions: [] };
    if (fs.existsSync(storagePath)) {
        const fileContent = fs.readFileSync(storagePath, 'utf-8');
        storageData = JSON.parse(fileContent);
    }

    // 查找或创建session
    let session = storageData.sessions.find((s: any) => s.session_id === options.sessionId);
    if (!session) {
        session = {
            session_id: options.sessionId,
            status: 0,
            create_at: new Date().toISOString(),
            entries: [],
            compacted_entries: []
        };
        storageData.sessions.push(session);
    }

    // 在存储前生成UUID
    const entryId = uuidv4();

    // 创建新的entry
    const newEntry: any = {
        entry_id: entryId,
        role: options.role,
        content: options.content,
        create_at: options.createAt || new Date().toISOString()
    };

    // 如果有tools信息，添加tools字段
    if (options.tools) {
        newEntry.tools = options.tools;
    }

    // 如果有token_consumption，添加该字段
    if (options.tokenConsumption !== undefined) {
        newEntry.token_consumption = options.tokenConsumption;
    }

    // 添加到entries数组
    session.entries.push(newEntry);

    // 写回文件
    fs.writeFileSync(storagePath, JSON.stringify(storageData, null, 2), 'utf-8');
}


/**
 * 作用：将压缩记录写入本地JSON文件
 * 关联：被storage/index.ts调用
 * 预期结果：将压缩记录追加到.veloca/storage/veloca-storage.json文件中
 */
export function WriteCompactedEntryToFile(options: {
    sessionId: string;
    summary: string;
    triggerEntryId: string;
    createAt?: string;
}): void {
    // 获取存储文件路径
    const storagePath = path.join(process.cwd(), '.veloca', 'storage', 'veloca-storage.json');
    
    // 确保目录存在
    const storageDir = path.dirname(storagePath);
    if (!fs.existsSync(storageDir)) {
        fs.mkdirSync(storageDir, { recursive: true });
    }
    
    // 读取现有数据
    let storageData: any = { sessions: [] };
    if (fs.existsSync(storagePath)) {
        const fileContent = fs.readFileSync(storagePath, 'utf-8');
        storageData = JSON.parse(fileContent);
    }
    
    // 查找或创建session
    let session = storageData.sessions.find((s: any) => s.session_id === options.sessionId);
    if (!session) {
        session = {
            session_id: options.sessionId,
            create_at: new Date().toISOString(),
            entries: [],
            compacted_entries: []
        };
        storageData.sessions.push(session);
    }
    
    // 在存储前生成UUID
    const entryId = uuidv4();
    
    // 创建新的压缩记录
    const newCompactedEntry = {
        entry_id: entryId,
        summary: options.summary,
        trigger_entry_id: options.triggerEntryId,
        create_at: options.createAt || new Date().toISOString()
    };
    
    // 添加到compacted_entries数组
    session.compacted_entries.push(newCompactedEntry);
    
    // 写回文件
    fs.writeFileSync(storagePath, JSON.stringify(storageData, null, 2), 'utf-8');
}
