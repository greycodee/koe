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
import { open } from '@tauri-apps/plugin-dialog';
import { appLocalDataDir } from '@tauri-apps/api/path';
import {Channel,convertFileSrc, invoke} from "@tauri-apps/api/core";

import Database from '@tauri-apps/plugin-sql';
import { Badge } from "@/components/ui/badge";


type History = {
    id            :number,
    src_path      :string,
    src_file_name :string,
    out_file_name :string,
    status        :"Pending" | "Success" | "Failed" ,
    crate_at      :number
}


type EventType =
    | {
    event: 'started';
    data: {
        url: string;
        downloadId: number;
        contentLength: number;
    };
}
    | {
    event: 'progress';
    data: {
        val: number;
    };
}
    | {
    event: 'finished';
    data: {
        downloadId: number;
    };
};

export default function App() {
    const [history, setHistory] = useState<History[]>([]);
    const [sqliteDB, setSqliteDB] = useState<Database>();
    const [localDataDir, setLocalDataDir] = useState<string>("");

    // let localDataDir = await appLocalDataDir();
    // let mp3FilePath = await join(localDataDir, mp3FileName);
    // let mp3FileUrl = convertFileSrc(mp3FilePath);

    useEffect(() => {

        (async () => {
            let localDataDir = await appLocalDataDir();
            setLocalDataDir(localDataDir);
            const db = await Database.load('sqlite:koe.db');
            if (!db) {
                console.error("Failed to load database");
                return;
            }else {
                setSqliteDB(db);
                db.select<History[]>("SELECT * FROM history").then((result) => {
                    console.log("result:", result);
                    setHistory(result);
                });
                console.log("history:", history);
            }
            // const history = await db.execute("SELECT * FROM history").fetch_all();
        })();
    }, []);



    const onEvent = new Channel<EventType>();
    onEvent.onmessage = (message) => {
        // console.log(`got download event ${message.data?.chunkLength}`);
        if (message.event === 'progress') {
            console.log(`got download event ${message.data.val}`);
            // setProgress(message.data.val);
        }

    };


    const handleAdd = async () => {

        const file = await open({
            multiple: false,
            directory: false,
            filters: [
                {
                    name: "Audio File",
                    extensions: [".arm","silk"]
                }
            ]
        });


        if (file) {
            const mp3FileName:string = await invoke("read_file", { path: file,onEvent });
            setHistory([...history, {
                id: history.length + 1,
                src_path: file,
                src_file_name: file.split("/").pop() as string,
                out_file_name: mp3FileName,
                crate_at: Date.now(),
                status: "Success",
            }]);
            sqliteDB?.execute("INSERT INTO history (src_path,src_file_name,out_file_name,crate_at,status) VALUES (?,?,?,?,?)", [file, file.split("/").pop(), mp3FileName, Date.now(), "Success"]);
        }
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
                                    <Button variant="destructive">Delete</Button>
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