import Database from '@tauri-apps/plugin-sql';

export type History = {
    id: number,
    src_path: string,
    src_file_name: string,
    out_file_name: string,
    status: "Pending" | "Success" | "Failed",
    crate_at: number
}

export class HistoryDAO {
    private db: Database | null = null;

    async initialize(): Promise<void> {
        try {
            this.db = await Database.load('sqlite:koe.db');
        } catch (error) {
            console.error("Failed to initialize database:", error);
            throw error;
        }
    }

    async getAllHistory(): Promise<History[]> {
        if (!this.db) throw new Error("Database not initialized");
        return await this.db.select<History[]>("SELECT * FROM history");
    }

    async addHistory(history: Omit<History, 'id'>): Promise<void> {
        if (!this.db) throw new Error("Database not initialized");
        await this.db.execute(
            "INSERT INTO history (src_path, src_file_name, out_file_name, crate_at, status) VALUES (?, ?, ?, ?, ?)",
            [history.src_path, history.src_file_name, history.out_file_name, history.crate_at, history.status]
        );
    }

    async deleteHistory(id: number): Promise<void> {
        if (!this.db) throw new Error("Database not initialized");
        await this.db.execute("DELETE FROM history WHERE id = ?", [id]);
    }
}

// Create a singleton instance
export const historyDAO = new HistoryDAO();
