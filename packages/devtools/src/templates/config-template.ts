import { uuid } from "../common/utils";


export type ConfigType = 'simple' | 'vueVite' | 'vue' | 'react';

export function generateConfig(type: ConfigType, name: string) {
	return ({
		simple: {
			name,
			uuid: uuid(),
		},

		vueVite: {
			name,
			uuid: uuid(),

			debug: {
				entry: "http://localhost:5173",
				resource: "public",
			},

			build: {
				resource: "dist",
			}
		},

		vue: {
			name,
			uuid: uuid(),

			debug: {
				entry: "http://localhost:8080",
				resource: "public",
			},

			build: {
				resource: "dist",
			}
		},

		react: {
			name,
			uuid: uuid(),

			debug: {
				entry: "http://localhost:3000",
				resource: "public",
			},

			build: {
				resource: "build",
			}
		}
	})[type];
}
