import { betterAuth } from "better-auth"
import Database from "better-sqlite3"
import { NodeSqliteDialect } from "@better-auth/kysely-adapter/node-sqlite-dialect"
import * as dotenv from "dotenv"

dotenv.config()

const db = new Database("./data/auth.db")

export const auth = betterAuth({
    database: new NodeSqliteDialect({ database: db }),
    socialProviders: {
        github: {
            clientId: import.meta.env.OAUTH_GITHUB_CLIENT_ID,
            clientSecret: import.meta.env.OAUTH_GITHUB_CLIENT_SECRET,
            scope: ["user:email"]
        },
    },
    advanced: {
        useSecureCookies: true
    }
})