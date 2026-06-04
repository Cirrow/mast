import type { APIRoute } from "astro";
import { auth } from "../../utils/auth"
import { savePage } from "../../utils/git-service";
import { checkRateLimit, RateLimitError } from "../../utils/rate-limit";

export const POST: APIRoute = async ({request}) => {
    let body: { path: string; content: string; sha: string; }

    try {
        body = await request.json()
    } catch {
        return new Response(JSON.stringify({ error: "invalid JSON body"}), {
            status: 400,
            headers: { "Content-Type": "application/json"}
        })
    }

    const { path, content } = body;
  
    if (!path || content === undefined) {
        return new Response(
            JSON.stringify({ error: "missing required fields: path, content" }),
            { status: 400, headers: { "Content-Type": "application/json" } }
        );
    }
  
        try {
            const session = await auth.api.getSession({ headers: request.headers });
            if (!session) throw new Error("Unauthorized");
            
            await checkRateLimit(session.user.id, "/api/save", { max: 10, windowSec: 60 });
            
            const contentDir = process.env.CONTENT_DIR || ".wiki/wiki";
            const filePath = `${contentDir}/${path.replace(/^wiki\//, "")}`;
            const newSha = await savePage(
                filePath,
                content,
                session.user.name,
                session.user.email
            );
            
            return new Response(JSON.stringify({ ok: true, sha: newSha }), {
                status: 200,
                headers: { "Content-Type": "application/json" },
            });

        } catch (e: any) {
            if (e instanceof RateLimitError) {
                return new Response(
                    JSON.stringify({ error: `rate limit exceeded. try again in ${e.retryAfter}s` }),
                    { status: 429, headers: { "Content-Type": "application/json" } }
            );
        }

        if (e?.message === "Unauthorized") {
            return new Response(JSON.stringify({ error: "unauthorized" }), {
                status: 401,
                headers: { "Content-Type": "application/json" },
            });
        }
    
        console.error("[/api/save] error:", e);
    
        return new Response(
            JSON.stringify({ error: e?.message ?? "internal error" }),
            { status: 500, headers: { "Content-Type": "application/json" } }
        )
    }
}