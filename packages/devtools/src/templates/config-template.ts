import { uuid } from "../common/utils";


export type ConfigType = 'simple' | 'vueVite' | 'vue' | 'react';

export function generateConfig(type: ConfigType, name: string) {
	return ({
		simple: {
			name,
			uuid: uuid(),
			injectCommonJs: false,
			injectEsm: false,
		},

		vueVite: {
			name,
			uuid: uuid(),
			injectCommonJs: false,
			injectEsm: false,

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
			injectCommonJs: false,
			injectEsm: false,

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
			injectCommonJs: false,
			injectEsm: false,

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
