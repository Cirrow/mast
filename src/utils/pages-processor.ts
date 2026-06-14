// Wiki processing pipeline that transforms raw wiki content into HTML. Pages-processor (PP)


import Lexer  from '../scripts/Lexing/lexer';
import Parser from '../scripts/Parsing/parser';
import Renderer from '../scripts/Rendering/renderer';

export interface RawPage {
    filePath: string,
    content: string,
    lastModified: Date,
    size: number
}


// Configuration for page processing
interface PageProcessorConfig {
    enableCaching: boolean;
    validateOutput: boolean;
    stripEmptyLines: boolean;
    generateTOC: boolean; // Table of Contents
    maxProcessingTime: number; // milliseconds
}

// Processed page with HTML output
interface ProcessedPage {
    filePath: string;
    rawContent: string;
    htmlContent: string;
    title: string;
    excerpt: string;
    toc?: TOCItem[];
    metadata: {
        lastModified: Date;
        size: number;
        processingTime: number;
        wordCount: number;
    };
}

// Table of contents structure
export interface TOCItem {
    level: number;
    title: string;
    anchor: string;
    children?: TOCItem[];
}

// Processing statistics
interface ProcessingStats {
    totalPages: number;
    successfulPages: number;
    failedPages: number;
    totalProcessingTime: number;
    averageProcessingTime: number;
    errors: ProcessingError[];
}

// Processing error details
interface ProcessingError {
    filePath: string;
    error: Error;
    stage: 'lexing' | 'parsing' | 'rendering' | 'validation';
}


// Custom error class for processing errors
export class PageProcessorError extends Error {
    constructor(
        message: string,
        public readonly stage: string,
        public readonly filePath: string,
        public readonly cause?: Error
    ) {
        super(message);
        this.name = 'PageProcessorError';
    }
}

const defaultConfig: PageProcessorConfig = {
    enableCaching: true,
    validateOutput: true,
    stripEmptyLines: true,
    generateTOC: true,
    maxProcessingTime: 7000, // 7 seconds per page
};


// Extract title from wiki content (first heading or filename)
function extractTitle( content: string ): string {
    // Try to find first heading in content
const headingMatch = content.match(/^=+\s*(.+?)\s*=+\s*$/m);
    if (headingMatch) {
        return headingMatch[1].trim();
    } else {
        throw new Error("No h1 found in files")
    }
}


// Extract excerpt from content (first paragraph or first N characters)

function extractExcerpt(content: string, maxLength: number = 200): string {
    // Remove headings and empty lines
    const cleanContent = content
        .replace(/^#+\s+.+$/gm, '') // Remove headings
        .replace(/^\s*$/gm, '') // Remove empty lines
        .trim();
    
    // Get first paragraph or truncate
    const firstParagraph = cleanContent.split('\n\n')[0] || cleanContent;
    
    if (firstParagraph.length <= maxLength) {
        return firstParagraph;
    }
    
    return firstParagraph.substring(0, maxLength).replace(/\s+\S*$/, '') + '...';
}


// Count words in text content
function countWords(text: string): number {
    return text.trim().split(/\s+/).filter(word => word.length > 0).length;
}


// Generate table of contents from HTML headings
function generateTOC( htmlContent: string ): TOCItem[] {
    const headings: TOCItem[] = [];
    const headingRegex = /<div[^>]*><h([1-6])[^>]*id="([^"]*)"[^>]*>(?:<span[^>]*>)?([^<]+)(?:<\/span>)?<\/h[1-6]><\/div>/gi;    let match;
    
    while ((match = headingRegex.exec(htmlContent)) !== null) {

        const level = parseInt(match[1]);
        const anchor = match[3].trim().toLowerCase().replace(/[^a-zA-Z0-9 ]/g, "").replace(/ /g, "_")
        const title = match[3].trim();
        
        headings.push({
            level,
            title,
            anchor,
        })
    }
    
    return headings
}

// Validate HTML output for common issues
function validateHtmlOutput(html: string, filePath: string): void {
    // Check for unclosed tags (basic validation)
    const openTags = (html.match(/<[^\/][^>]*>/g) || []).length;
    const closeTags = (html.match(/<\/[^>]*>/g) || []).length;
    const selfClosingTags = (html.match(/<[^>]*\/>/g) || []).length;
    
    // super basic check, in real scenarios we might want proper HTML validation -- TODO
    if (Math.abs(openTags - closeTags - selfClosingTags) > 5) {
        console.warn(`LORA PP: Potential HTML structure issues in ${filePath}`);
    }
    
    // Check for empty output
    if (html.trim().length === 0) {
        throw new PageProcessorError(
            'Generated HTML is empty',
            'validation',
            filePath
        );
    }
}


// Process a single wiki page through the lexer → parser → renderer pipeline
export function processPage( rawPage: RawPage, config: Partial<PageProcessorConfig> = {} ): ProcessedPage {
    const finalConfig = { ...defaultConfig, ...config };
    const startTime = Date.now()
    const lexer = new Lexer()
    const parser = new Parser()
    const renderer = new Renderer()

    
    try {
        let content = rawPage.content
        
        // Preprocessing
        if (finalConfig.stripEmptyLines) {
            content = content.replace(/^\s*\n/gm, '');
        }
        // Stage 1: Lexical Analysis
        let tokens;
        try {
            tokens = lexer.tokenise(content);
        } catch (e) {
            throw new PageProcessorError( 'LORA: Lexical analysis failed', 'lexing', rawPage.filePath, e as Error )
        }
        // Stage 2: Parsing
        let ast;
        try {
            ast = parser.parse(tokens);
        } catch (e) {
            throw new PageProcessorError( 'LORA PP: Parsing failed', 'parsing', rawPage.filePath, e as Error )
        }
        // Stage 3: Rendering
        let htmlContent;
        try {
            htmlContent = renderer.render(ast);
        } catch (e) {
            throw new PageProcessorError( 'LORA PP: Rendering failed', 'rendering', rawPage.filePath, e as Error )
        }
        
        // Stage 4: Validation
        if (finalConfig.validateOutput) {
            try {
                validateHtmlOutput(htmlContent, rawPage.filePath);
            } catch (e) {
                throw new PageProcessorError( 'LORA PP: HTML validation failed', 'validation', rawPage.filePath, e as Error )
            }
        }

        // Extract metadata
        const title = extractTitle(rawPage.content);
        const excerpt = extractExcerpt(rawPage.content);
        const wordCount = countWords(rawPage.content);
        const processingTime = Date.now() - startTime;

        
        // Generate table of contents
        let toc: TOCItem[] | undefined;
        if (finalConfig.generateTOC) {
            toc = generateTOC(htmlContent);
        }
        
        // Check processing time
        if (processingTime > finalConfig.maxProcessingTime) {
            console.warn(`LORA PP: Page ${rawPage.filePath} took ${processingTime}ms to process`);
        }

        return {
            filePath: rawPage.filePath,
            rawContent: rawPage.content,
            htmlContent,
            title,
            excerpt,
            toc,
            metadata: {
                lastModified: rawPage.lastModified,
                size: rawPage.size,
                processingTime,
                wordCount,
            },
        };
        
    } catch (e) {
        if (e instanceof PageProcessorError) {
            throw e;
        }
        
        throw new PageProcessorError(
            'LORA PP: Unexpected error during page processing',
            'unknown',
            rawPage.filePath,
            e as Error
        );
    }
}


export async function processAllPages( rawPages: RawPage[], config: Partial<PageProcessorConfig> = {} ): Promise<{ processedPages: ProcessedPage[]; stats: ProcessingStats; }> {
    const finalConfig = { ...defaultConfig, ...config };
    const startTime = Date.now();
    
    console.log(`LORA PP: Processing ${rawPages.length} wiki pages...`);
    
    const processedPages: ProcessedPage[] = [];
    const errors: ProcessingError[] = [];
    
    // Process pages in parallel with error isolation
    const results = await Promise.allSettled(
        rawPages.map(rawPage => processPage(rawPage, finalConfig))
    )
    
    // Collect results and errors
    results.forEach((result, index) => {
        if (result.status === 'fulfilled') {
            processedPages.push(result.value);
        } else {
            const rawPage = rawPages[index];
            const error = result.reason;
            
            errors.push({
                filePath: rawPage.filePath,
                error: error instanceof Error ? error : new Error(String(error)),
                stage: error instanceof PageProcessorError ? error.stage as any : 'unknown',
            });
            
            console.error(`LORA PP: Failed to process ${rawPage.filePath}:`, error);
        }
    });
    
    const totalProcessingTime = Date.now() - startTime;
    const stats: ProcessingStats = {
        totalPages: rawPages.length,
        successfulPages: processedPages.length,
        failedPages: errors.length,
        totalProcessingTime,
        averageProcessingTime: processedPages.length > 0 
            ? processedPages.reduce((sum, page) => sum + page.metadata.processingTime, 0) / processedPages.length 
            : 0,
        errors,
    };
    
    console.log(`LORA PP: Processing complete! ${stats.successfulPages}/${stats.totalPages} pages successful in ${totalProcessingTime}ms`);
    
    if (errors.length > 0) {
        console.warn(`LORA PP: ${errors.length} pages failed to process`);
    }
    
    return { processedPages, stats };
}


// Process pages with caching support (in-memory for build time)
export async function processWithCache(
    rawPages: RawPage[],
    config: Partial<PageProcessorConfig> = {}
): Promise<{
    processedPages: ProcessedPage[];
    stats: ProcessingStats;
}> {
    const finalConfig = { ...defaultConfig, ...config };
    
    if (!finalConfig.enableCaching) {
        return processAllPages(rawPages, finalConfig);
    }
    
    // Simple in-memory cache for build time (could extend to file-based caching TODO)
    const cache = new Map<string, ProcessedPage>();
    const cacheHits = [];
    const cacheMisses = [];
    
    for (const rawPage of rawPages) {
        const cacheKey = `${rawPage.filePath}:${rawPage.lastModified.getTime()}`;
        
        if (cache.has(cacheKey)) {
            cacheHits.push(rawPage);
        } else {
            cacheMisses.push(rawPage);
        }
    }
    
    console.log(`LORA PP: Cache stats - ${cacheHits.length} hits, ${cacheMisses.length} misses`);
    
    // Process only cache misses
    const { processedPages: newProcessedPages, stats } = await processAllPages(cacheMisses, finalConfig);
    
    // Update cache
    newProcessedPages.forEach(page => {
        const cacheKey = `${page.filePath}:${page.metadata.lastModified.getTime()}`;
        cache.set(cacheKey, page);
    });
    
    // Combine cached and newly processed pages
    const allProcessedPages = [
        ...cacheHits.map(rawPage => {
            const cacheKey = `${rawPage.filePath}:${rawPage.lastModified.getTime()}`;
            return cache.get(cacheKey)!;
        }),
        ...newProcessedPages,
    ];
    
    return {
        processedPages: allProcessedPages,
        stats: {
            ...stats,
            totalPages: rawPages.length,
            successfulPages: allProcessedPages.length,
        },
    };
}