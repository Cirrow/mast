import { parse } from 'smol-toml'
import fs from 'fs'
import path from 'path'
import { fileURLToPath } from 'url'
import { z } from 'zod'

const ConfigSchema = z.object({

    basic: z.object({
        name: z.string(),
        image_as_home: z.boolean()
    })


})


const __dirname = path.dirname(fileURLToPath(import.meta.url))
const raw = parse(fs.readFileSync(path.join(__dirname, '../mast-config.toml'), 'utf8'))


const result = ConfigSchema.safeParse(raw)
if (!result.success) {
    console.error('Invalid config:', result.error.format())
    process.exit(1)
}

const config = result.data

export default config