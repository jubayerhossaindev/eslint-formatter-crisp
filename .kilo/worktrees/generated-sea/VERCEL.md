# Vercel Deployment

This project uses Vercel for documentation deployment.

## Setup

1. Import this repository to Vercel
2. Vercel will auto-detect Next.js in `apps/docs`
3. Build command: `cd apps/docs && npm run build`
4. Output directory: `apps/docs/.next`

## Environment Variables

No environment variables required for static build.

## Manual Deployment

```bash
cd apps/docs
npm run build
```
