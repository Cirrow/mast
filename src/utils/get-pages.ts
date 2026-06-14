import type { RawPage } from "./pages-processor";
import { readdir, readFile, stat } from 'fs/promises';
import * as path from 'path';
import { existsSync } from "fs"

import config from "#config";

export async function getAllPages(pPath: string) {

    if (!existsSync(config.storage.location)) {
        throw new Error(`Local wiki storage not found. Double-check if selected wiki raw files directory ${config.storage.location} exists.`)
    }

    const rawPages: RawPage[] = [];

    try {
        const absolutePath = path.resolve(pPath);
        const entries = await readdir(absolutePath, { withFileTypes: true });

        for (const entry of entries) {
            
            if (entry.isFile()) {
                const fullPath = path.join(absolutePath, entry.name);

                // 4. Fetch the file data and metadata concurrently
                // (Using Promise.all makes this faster as they run at the same time)
                const [content, fileStats] = await Promise.all([
                    readFile(fullPath, 'utf-8'),
                    stat(fullPath)
                ]);

                // 5. Push the structured object into your array
                rawPages.push({
                    filePath: fullPath,             // Absolute path to the file
                    content: content,               // File text data
                    lastModified: fileStats.mtime,  // .mtime is the "Modification Time" Date object
                    size: fileStats.size            // File size in bytes
                });
            }
        }
  } catch (error) {
    console.error('Failed to compile RawPages:', error);
    // Depending on your architecture, you might want to rethrow the error here
  }
    
    
  return rawPages

}