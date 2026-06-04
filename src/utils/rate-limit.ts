import * as dotenv from "dotenv"
dotenv.config()
export class RateLimitError extends Error {
    retryAfter: number;
    constructor(retryAfter: number) {
        super("rate limit exceeded");
        this.retryAfter = retryAfter;
        this.name = "RateLimitError";
    }
}
const hits = new Map<string, number[]>();
export async function checkRateLimit(
    userId: string,
    endpoint: string,
    opts: { max: number; windowSec: number }
): Promise<void> {
    const key = `${userId}:${endpoint}`;
    const now = Date.now();
    const windowMs = opts.windowSec * 1000;
    let timestamps = hits.get(key) || [];
    timestamps = timestamps.filter(t => now - t < windowMs);
    if (timestamps.length >= opts.max) {
        throw new RateLimitError(opts.windowSec);
    }
    timestamps.push(now);
    hits.set(key, timestamps);
}