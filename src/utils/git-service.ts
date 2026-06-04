// fetches raw wiki files from remote git at compile time. Git-service (GS)

import global from '../store';

import { execSync } from 'child_process';
import fs from 'fs/promises';
import path from 'path';


// Configuration for content fetching
export interface ContentFetchConfig {
    repoUrl: string;
    branch: string;
    localPath: string;
    wikiContentPath: string;
    gitTimeout: number; // Timeout for git operations in milliseconds
    maxFileSize: number; // Maximum file size in bytes
}

// Raw page (.txt files saved in github repo) metadata
export interface RawPage {
    slug: string[],
    filePath: string,
    content: string,
    lastModified: Date,
    size: number
}

export interface PageMeta {
    id: string
    title: string
    slug: string[]
    filePath: string
    lastModified: Date
    size: number
}


const defaultConfig: ContentFetchConfig = {
    repoUrl: 'https://github.com/lorearchive/law-content.git', //might change to SSH?
    branch: 'main',
    localPath: '.wiki', // Where to store the cloned repo
    wikiContentPath: 'wiki', // Path within the repo containing wiki files
    gitTimeout: 30000, // 30 seconds
    maxFileSize: 10 * 1024 * 1024, // 10MB
}

// Custom error classes for better error handling
export class GitServiceError extends Error {
    constructor(message: string, public readonly cause?: Error) {
        super(message);
        this.name = 'GitServiceError';
    }
}

export class ContentValidationError extends GitServiceError {
    constructor(message: string, cause?: Error) {
        super(message, cause);
        this.name = 'ContentValidationError';
    }
}


// check if a directory exists
async function directoryExists(dirPath: string): Promise<boolean> {
    try {
        const stat = await fs.stat(dirPath);
        return stat.isDirectory();
    } catch {
        return false;
    }
}


// Validate and sanitize file paths to prevent directory traversal
function sanitizePath(inputPath: string): string {
    // Remove any path traversal attempts
    const sanitized = path.normalize(inputPath).replace(/^(\.\.[\/\\])+/, '')
    return sanitized
}


// Execute git command with timeout and error handling
function executeGitCommand(command: string, timeout: number): void {
    try {
        execSync(command, {
            stdio: 'pipe', // Capture output instead of inheriting
            timeout,
            encoding: 'utf8',
        })

    } catch (e: any) {
        throw new GitServiceError( `LORA: Git command failed: ${command}`, e );
    }
}

/**
 * Fetch wiki content from a remote Git repository
 * @param config Fetching configuration
 * @returns Path to the local content directory
 */

export async function fetchWikiContent( config: Partial<ContentFetchConfig> = {} ): Promise<string> {
    const finalConfig = { ...defaultConfig, ...config };
    const contentPath = process.env.CONTENT_DIR || path.join(finalConfig.localPath, finalConfig.wikiContentPath);
    
    if (!(await directoryExists(contentPath))) {
        console.warn(`LORA: Content directory '${contentPath}' not found. Creating...`);
        await fs.mkdir(contentPath, { recursive: true });
    }
    
    console.log(`LORA: Using wiki content from: ${contentPath}`);
    return contentPath;
}

// validate file size and content 
async function validateWikiFile(filePath: string, maxSize: number): Promise<void> {
    try {
        const stats = await fs.stat(filePath);
    
        if (stats.size > maxSize) {
            throw new ContentValidationError( `LORA: File '${filePath}' exceeds maximum size limit (${maxSize} bytes)` );
        }
    
        if (stats.size === 0) {
            console.warn(`LORA: Warning: Empty file found: ${filePath}`);
        }
    
    } catch (e) {
        if (e instanceof ContentValidationError) {
            throw e;
        }
        throw new ContentValidationError(`LORA: Cannot access file: ${filePath}`, e as Error);
    }
}


// Generate URL-safe slug from file path components
function generateSlug(pathComponents: string[]): string[] {
    return pathComponents.map(component => 
        component
            .toLowerCase()
            .replace(/\s+/g, '-')
            .replace(/[^\w\-]/g, '')
            .replace(/--+/g, '-')
            .replace(/^-+|-+$/g, '')
    ).filter(Boolean)
}

// Helper function to get the last commit date for a specific file
async function getLastCommitDate(filePath: string, repoPath: string): Promise<Date> {
    try {
        const relativePath = path.relative(repoPath, filePath);
        const command = `cd "${repoPath}" && git log -1 --format="%ci" -- "${relativePath}"`;
        
        console.log(`LORA: Running git command: ${command}`);
        
        const result = execSync(command, {
            stdio: 'pipe',
            timeout: 5000,
            encoding: 'utf8',
        });
        
        const commitDateStr = result.trim();
        
        if (!commitDateStr) {
            // If no commit found for this file, fall back to file modification time
            console.warn(`LORA: No git history found for ${relativePath}, using file modification time`);
            const stats = await fs.stat(filePath);
            return stats.mtime;
        }
        
        const commitDate = new Date(commitDateStr);
        return commitDate;
    } catch (e) {
        console.warn(`LORA: Error getting git commit date for ${filePath}:`, e);
        // fall back to file modification time
        try {
            const stats = await fs.stat(filePath);
            console.log(`LORA: [DEBUG] Fallback to file mtime for ${filePath}: ${stats.mtime.toISOString()}`);
            return stats.mtime;
        } catch (statError) {
            // If we can't even get file stats, return current date as last resort
            console.error(`LORA: Cannot get file stats for ${filePath}:`, statError);
            return new Date();
        }
    }
}

async function checkMetaDir( wikiPath: string ): Promise<string> {
    const metaDir = path.join(wikiPath, 'meta');
    
    try {
        await fs.access(metaDir);
    } catch {
        await fs.mkdir(metaDir, { recursive: true });
        console.log(`LORA: Created metadata directory: ${metaDir}`);
    }
    
    return metaDir;
}

async function savePageMetadata(metaDir: string, metadata: PageMeta): Promise<void> {
    const fileName = metadata.id + '.json';
    const filePath = path.join(metaDir, fileName);
    
    await fs.writeFile(filePath, JSON.stringify(metadata, null, 2), 'utf-8');
}

/**
 * Get all wiki pages from the content directory
 * @param contentPath Path to the local content directory
 * @param config Configuration options
 * @returns Array of objects with page paths and content
 */
export async function getAllPages( contentPath: string, config: Partial<Pick<ContentFetchConfig, 'maxFileSize'>> = {} ): Promise<RawPage[]> {

    const finalConfig = { ...defaultConfig, ...config };
    const pages: RawPage[] = [];

    // Validate content path
    if (!(await directoryExists(contentPath))) {
        throw new ContentValidationError(`LORA: Content directory does not exist: ${contentPath}`);
    }

    const wikiPath = path.dirname(contentPath); // Get .wiki directory
    const metaDir = await checkMetaDir(wikiPath);


    // Find the git repository root (should be the parent of contentPath)
    const repoPath = path.dirname(contentPath);
    const gitDirExists = await directoryExists(path.join(repoPath, '.git'));
    if (!gitDirExists) {
        console.warn('LORA: Git repository not found, falling back to file modification times');
    }

    // Recursive function to walk directory tree
    async function processDirectory(dirPath: string, parentSlug: string[] = []) {
        try {
            const entries = await fs.readdir(dirPath, { withFileTypes: true });
      
            // Process entries in parallel for better performance
            await Promise.all( entries.map(async (entry) => {
                
                const entryPath = path.join(dirPath, entry.name);
          
                if (entry.isDirectory()) {
                    // Skip hidden directories and common ignore patterns
                    if (entry.name.startsWith('.') || entry.name === 'node_modules') return
            
                    // Process subdirectory recursively
                    await processDirectory(entryPath, [...parentSlug, entry.name])
            
                } else if (entry.isFile() && entry.name.endsWith('.txt')) {
            
                    try {
                        await validateWikiFile(entryPath, finalConfig.maxFileSize);
              
                        const pageName = entry.name.replace(/\.txt$/, '');
                        const slug = generateSlug([...parentSlug, pageName]);
              
                        const content = await fs.readFile(entryPath, 'utf-8');
                        const stats = await fs.stat(entryPath);
                        
                        const lastModified = gitDirExists 
                            ? await getLastCommitDate(entryPath, repoPath)
                            : stats.mtime;

                       const slugPath = slug.join("/");
                        global.lastMod[slugPath] = lastModified;

                        const title = content.split('\n')[0].replace(/^#+\s*/, '') || pageName;

                        const pageData = { 
                            slug,
                            filePath: entryPath,
                            content: content.trim(),
                            lastModified,
                            size: stats.size
                        };

                        const metadata: PageMeta = {
                            id: pageName,
                            title: title.trim().slice(6).slice(0, -6).trim(),
                            slug,
                            filePath: entryPath,
                            lastModified,
                            size: stats.size
                        };

                        await savePageMetadata(metaDir, metadata);
                                    
                        pages.push(pageData);
              
                    } catch (e) {
                        console.error(`LORA: Error processing wiki file '${entryPath}':`, e);
                        // Continue processing other files instead of failing completely
                    }
                } else {
                    console.log(`LORA: Unrecognized entry: ${entry.name}`)
                }
            }));
      
        } catch (e) {
            throw new ContentValidationError(`LORA: Error reading directory '${dirPath}'`, e as Error);
        }
    }

    await processDirectory(contentPath);
  
    console.log(`LORA: Successfully processed ${pages.length} wiki pages`);
    
    return pages;
}

/**
 * Get a single wiki page by its slug path
 * @param contentPath Path to the local content directory
 * @param slug Array representing the page path
 * @returns Page object or null if not found
 */
export async function getSinglePage( contentPath: string, slug: string[] ): Promise<RawPage | null> {
    try {
        const filePath = path.join(contentPath, ...slug) + '.txt';
    
        // Validate the constructed path is within content directory
        const resolvedPath = path.resolve(filePath);
        const resolvedContentPath = path.resolve(contentPath);
        
        if (!resolvedPath.startsWith(resolvedContentPath)) {
            throw new ContentValidationError('LORA: Invalid file path: outside content directory');
        }
    
        await validateWikiFile(filePath, defaultConfig.maxFileSize);
    
        const [content, stats] = await Promise.all([
            fs.readFile(filePath, 'utf-8'),
            fs.stat(filePath)
        ]);
    
        return { slug: generateSlug(slug), filePath, content: content.trim(), lastModified: stats.mtime, size: stats.size };
    
    } catch (e) {
        if (e instanceof ContentValidationError) throw e    
        return null // Page not found
    }
}

/**
 * Clean up local repository (useful for development/testing)
 * @param config Configuration options
 */
export async function cleanupLocalRepo( config: Partial<ContentFetchConfig> = {} ): Promise<void> {
  
    const finalConfig = { ...defaultConfig, ...config };
    const localPath = sanitizePath(finalConfig.localPath);
  
    try {
        if (await directoryExists(localPath)) {
            await fs.rm(localPath, { recursive: true, force: true });
            console.log(`LORA: Cleaned up local repository: ${localPath}`);
        }
    } catch (e) {
        throw new GitServiceError(`LORA: Failed to cleanup local repository: ${localPath}`, e as Error);
    }
}

export async function savePage( 
    filePath: string,
    content: string,
    authorName: string,
    authorEmail: string
): Promise<string> {

    const fullPath = path.resolve(filePath)
    await fs.writeFile(fullPath, content, 'utf-8')

    const repoDir = path.dirname(fullPath)

    let gitRoot = repoDir
    while (gitRoot !== '/' && !(await directoryExists(path.join(gitRoot, ".git")))) {
        gitRoot = path.dirname(gitRoot)
    }

    const relativePath = path.relative(gitRoot, fullPath)

    execSync(`git add "${relativePath}"`, { cwd: gitRoot, stdio: 'pipe', timeout: 10000 });
    execSync(
        `git commit -m "${authorName} edited ${relativePath}" --author="${authorName} <${authorEmail}>"`,
        { cwd: gitRoot, stdio: 'pipe', timeout: 10000 }
    );

    const sha = execSync(`git log -1 --format="%H"`, { cwd: gitRoot, stdio: 'pipe', timeout: 5000, encoding: 'utf8' }).trim();
    return sha
}