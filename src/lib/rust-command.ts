import { Channel, invoke } from "@tauri-apps/api/core";
import { open,save } from '@tauri-apps/plugin-dialog';

export type EventType = {
    event: 'started' | 'progress' | 'finished';
    data: {
        url?: string;
        downloadId?: number;
        contentLength?: number;
        val?: number;
    };
};

export class RustCommands {
    static async openAudioFile() {
        return await open({
            multiple: false,
            directory: false,
            filters: [
                {
                    name: "Audio File",
                    extensions: [".arm", "silk"]
                }
            ]
        });
    }

    static async saveAudioFile(fileName: string) {
        const outPath = await save({
            defaultPath: fileName,
            filters: [{
                name: 'Audio File',
                extensions: ['mp3']
            }]
        });
        console.log("saveFilePath", outPath);
        await invoke("save_mp3_file", { fileName, outPath });
    }

    static async convertAudioFile(path: string, onEvent: Channel<EventType>): Promise<string> {
        return await invoke("read_file", { path, onEvent });
    }


    static async deleteMp3File(fileName: string) {
        return await invoke("delete_mp3_file", { fileName });
    }

}
