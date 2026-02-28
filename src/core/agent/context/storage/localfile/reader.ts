import * as fs from 'fs';
import * as path from 'path';

/**
 * 作用：读取本地存储文件
 * 关联：被localfile/index.ts调用，读取.veloca/storage目录下的JSON文件
 * 预期结果：返回解析后的存储数据对象
 */
export function ReadStorageFile(): any {
    // 构建存储文件路径
    const storagePath = path.join(process.cwd(), '.veloca', 'storage', 'veloca-storage.json');
    
    // 检查文件是否存在
    if (!fs.existsSync(storagePath)) {
        // 如果文件不存在，创建目录和初始文件
        const storageDir = path.dirname(storagePath);
        if (!fs.existsSync(storageDir)) {
            fs.mkdirSync(storageDir, { recursive: true });
        }
        
        // 创建初始空数据结构
        const initialData = {
            sessions: [],
            entries: [],
            compacted_entries: []
        };
        
        fs.writeFileSync(storagePath, JSON.stringify(initialData, null, 2), 'utf-8');
        return initialData;
    }
    
    // 读取文件内容
    const fileContent = fs.readFileSync(storagePath, 'utf-8');
    
    // 解析JSON
    const data = JSON.parse(fileContent);
    
    return data;
}

/**
 * 作用：根据session_id读取该会话的所有数据
 * 关联：被localfile/index.ts调用，读取指定会话的session、entries和compacted_entries
 * 预期结果：返回包含该会话所有相关数据的对象，如果session不存在则返回空数据结构
 */
export function ReadSessionData(sessionId: string): any {
    // 参数检查
    if (!sessionId) {
        throw new Error('sessionId is required');
    }
    
    // 读取完整存储数据
    const allData = ReadStorageFile();
    
    // 查找指定的session
    const session = allData.sessions.find((s: any) => s.session_id === sessionId && s.status === 0);
    
    // 如果session不存在，返回空数据结构（首次调用的情况）
    if (!session) {
        return {
            session: null,
            entries: [],
            compacted_entries: []
        };
    }
    
    // 从session中获取entries和compacted_entries
    const entries = session.entries || [];
    const compactedEntries = session.compacted_entries || [];
    
    return {
        session,
        entries,
        compacted_entries: compactedEntries
    };
}

/**
 * 作用：写入本地存储文件
 * 关联：被localfile/index.ts调用，将数据写入.veloca/storage目录
 * 预期结果：成功写入数据到文件
 */
export function WriteStorageFile(data: any): void {
    // 构建存储文件路径
    const storagePath = path.join(process.cwd(), '.veloca', 'storage', 'veloca-storage.json');
    
    // 确保目录存在
    const storageDir = path.dirname(storagePath);
    if (!fs.existsSync(storageDir)) {
        fs.mkdirSync(storageDir, { recursive: true });
    }
    
    // 写入文件
    fs.writeFileSync(storagePath, JSON.stringify(data, null, 2), 'utf-8');
}
