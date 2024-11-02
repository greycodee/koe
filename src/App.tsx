import { Button } from "@/components/ui/button"
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from "@/components/ui/table"
import {useState,useEffect} from "react";
import { appLocalDataDir } from '@tauri-apps/api/path';
import {Channel,convertFileSrc} from "@tauri-apps/api/core";

import { historyDAO, type History } from "@/lib/db-dao";
import { Badge } from "@/components/ui/badge";
import { RustCommands, type EventType } from "@/lib/rust-command";

export default function App() {
    const [history, setHistory] = useState<History[]>([]);
    const [localDataDir, setLocalDataDir] = useState<string>("");


    useEffect(() => {
        (async () => {
            let localDataDir = await appLocalDataDir();
            setLocalDataDir(localDataDir);
            await historyDAO.initialize();
            const result = await historyDAO.getAllHistory();
            setHistory(result);
        })();
    }, []);



    const onEvent = new Channel<EventType>();
    onEvent.onmessage = (message) => {
        if (message.event === 'progress') {
            console.log(`got download event ${message.data.val}`);
            // setProgress(message.data.val);
        }
    };


    const handleAdd = async () => {
        const file = await RustCommands.openAudioFile();

        if (file) {
            const mp3FileName = await RustCommands.convertAudioFile(file, onEvent);
            const newHistory: Omit<History, 'id'> = {
                src_path: file,
                src_file_name: file.split("/").pop() as string,
                out_file_name: mp3FileName,
                crate_at: Date.now(),
                status: "Success",
            };
            
            await historyDAO.addHistory(newHistory);
            const updatedHistory = await historyDAO.getAllHistory();
            setHistory(updatedHistory);
        }
    }

    const handleDelete = async (id: number,fileName:string) => {
        await historyDAO.deleteHistory(id);
        const updatedHistory = await historyDAO.getAllHistory();
        setHistory(updatedHistory);
        await RustCommands.deleteMp3File(fileName);
    }


    return (
        <div className="container mx-auto p-4 space-y-4">
            <div className="flex justify-between items-center flex-wrap gap-4">
                <h1 className="text-2xl font-bold">Convert</h1>
                <div>
                    <Button className="rounded-2xl" onClick={handleAdd}>Add</Button>
                </div>

            </div>
            <div className="overflow-x-auto">
                <Table>
                    <TableHeader>
                        <TableRow>
                            <TableHead>ID</TableHead>
                            <TableHead>FileName</TableHead>
                            <TableHead>Status</TableHead>
                            <TableHead>Action</TableHead>
                        </TableRow>
                    </TableHeader>
                    <TableBody>
                        {history.map((h) => (
                          <TableRow key={h.id}>
                                <TableCell>{h.id}</TableCell>
                                <TableCell>{h.out_file_name}</TableCell>
                                <TableCell>
                                    <Badge variant={h.status}>{h.status}</Badge>
                                </TableCell>
                                <TableCell>
                                    <Button variant="destructive">Download</Button>
                                    <Button variant="destructive"
                                        onClick={() => handleDelete(h.id,h.out_file_name)}
                                    >Delete</Button>
                                    <audio controls src={
                                        convertFileSrc(localDataDir+"/"+h.out_file_name)
                                    }>
                                        Your browser does not support the audio element.
                                    </audio>
                                </TableCell>
                            </TableRow>
                        ))}
                    </TableBody>
                </Table>
            </div>

        </div>
    )
}