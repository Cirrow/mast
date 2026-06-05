import { parse } from 'smol-toml'
import fs from 'fs'
import path from 'path'
import { z } from 'zod'

const ConfigSchema = z.object({

    basic: z.object({
        name: z.string(),
        image_as_home: z.boolean(),
        image_path: z.string().optional(),
        pinned_pages: z.array(z.string())

    }).superRefine((val, ctx) => {
        if (val.image_as_home && !val.image_path) {
            ctx.addIssue({
                code: 'custom',
                path: ['image_path'],
                message: 'MAST CONFIG: You must set an image path using image_path when you enable image_as_home.'
            })
        }

    }),

    auth: z.object({
        allow_signup: z.boolean().default(true)
    }),

    storage: z.object({
        type: z.enum(["local_git", "remote_git"]).default("local_git")
    })


})


const raw = parse(fs.readFileSync(path.join(process.cwd(), 'mast-config.toml'), 'utf8'))
const result = ConfigSchema.safeParse(raw)
if (!result.success) {
    console.error('Invalid config:', result.error.format())
    process.exit(1)
}

const config = result.data

export default config