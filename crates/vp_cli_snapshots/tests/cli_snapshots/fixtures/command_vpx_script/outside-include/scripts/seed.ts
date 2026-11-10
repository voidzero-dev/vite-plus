// scripts/ is outside the tsconfig include, so only the alias fallback resolves @app.
import { name } from '@app/name';

console.log('alias from scripts/:', name);
