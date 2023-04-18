"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[9776],{3905:(e,r,t)=>{t.d(r,{Zo:()=>c,kt:()=>y});var n=t(7294);function a(e,r,t){return r in e?Object.defineProperty(e,r,{value:t,enumerable:!0,configurable:!0,writable:!0}):e[r]=t,e}function i(e,r){var t=Object.keys(e);if(Object.getOwnPropertySymbols){var n=Object.getOwnPropertySymbols(e);r&&(n=n.filter((function(r){return Object.getOwnPropertyDescriptor(e,r).enumerable}))),t.push.apply(t,n)}return t}function o(e){for(var r=1;r<arguments.length;r++){var t=null!=arguments[r]?arguments[r]:{};r%2?i(Object(t),!0).forEach((function(r){a(e,r,t[r])})):Object.getOwnPropertyDescriptors?Object.defineProperties(e,Object.getOwnPropertyDescriptors(t)):i(Object(t)).forEach((function(r){Object.defineProperty(e,r,Object.getOwnPropertyDescriptor(t,r))}))}return e}function p(e,r){if(null==e)return{};var t,n,a=function(e,r){if(null==e)return{};var t,n,a={},i=Object.keys(e);for(n=0;n<i.length;n++)t=i[n],r.indexOf(t)>=0||(a[t]=e[t]);return a}(e,r);if(Object.getOwnPropertySymbols){var i=Object.getOwnPropertySymbols(e);for(n=0;n<i.length;n++)t=i[n],r.indexOf(t)>=0||Object.prototype.propertyIsEnumerable.call(e,t)&&(a[t]=e[t])}return a}var s=n.createContext({}),l=function(e){var r=n.useContext(s),t=r;return e&&(t="function"==typeof e?e(r):o(o({},r),e)),t},c=function(e){var r=l(e.components);return n.createElement(s.Provider,{value:r},e.children)},u="mdxType",d={inlineCode:"code",wrapper:function(e){var r=e.children;return n.createElement(n.Fragment,{},r)}},m=n.forwardRef((function(e,r){var t=e.components,a=e.mdxType,i=e.originalType,s=e.parentName,c=p(e,["components","mdxType","originalType","parentName"]),u=l(t),m=a,y=u["".concat(s,".").concat(m)]||u[m]||d[m]||i;return t?n.createElement(y,o(o({ref:r},c),{},{components:t})):n.createElement(y,o({ref:r},c))}));function y(e,r){var t=arguments,a=r&&r.mdxType;if("string"==typeof e||a){var i=t.length,o=new Array(i);o[0]=m;var p={};for(var s in r)hasOwnProperty.call(r,s)&&(p[s]=r[s]);p.originalType=e,p[u]="string"==typeof e?e:a,o[1]=p;for(var l=2;l<i;l++)o[l]=t[l];return n.createElement.apply(null,o)}return n.createElement.apply(null,t)}m.displayName="MDXCreateElement"},8399:(e,r,t)=>{t.r(r),t.d(r,{assets:()=>s,contentTitle:()=>o,default:()=>d,frontMatter:()=>i,metadata:()=>p,toc:()=>l});var n=t(7462),a=(t(7294),t(3905));const i={},o="\u6258\u76d8\u56fe\u6807 tray",p={unversionedId:"api/tray",id:"api/tray",title:"\u6258\u76d8\u56fe\u6807 tray",description:"Niva.api.tray.create",source:"@site/docs/api/tray.md",sourceDirName:"api",slug:"/api/tray",permalink:"/niva/en/docs/api/tray",draft:!1,tags:[],version:"current",frontMatter:{},sidebar:"docSidebar",previous:{title:"\u5168\u5c40\u5feb\u6377\u952e shortcut",permalink:"/niva/en/docs/api/shortcut"},next:{title:"Webview webview",permalink:"/niva/en/docs/api/webview"}},s={},l=[{value:"Niva.api.tray.create",id:"nivaapitraycreate",level:2},{value:"Niva.api.tray.destroy",id:"nivaapitraydestroy",level:2},{value:"Niva.api.tray.destroyAll",id:"nivaapitraydestroyall",level:2},{value:"Niva.api.tray.list",id:"nivaapitraylist",level:2},{value:"Niva.api.tray.update",id:"nivaapitrayupdate",level:2}],c={toc:l},u="wrapper";function d(e){let{components:r,...t}=e;return(0,a.kt)(u,(0,n.Z)({},c,t,{components:r,mdxType:"MDXLayout"}),(0,a.kt)("h1",{id:"\u6258\u76d8\u56fe\u6807-tray"},"\u6258\u76d8\u56fe\u6807 tray"),(0,a.kt)("h2",{id:"nivaapitraycreate"},"Niva.api.tray.create"),(0,a.kt)("pre",null,(0,a.kt)("code",{parentName:"pre",className:"language-ts"},"/**\n * \u5728\u7cfb\u7edf\u6258\u76d8\u4e2d\u521b\u5efa\u4e00\u4e2a\u65b0\u7684\u6258\u76d8\u56fe\u6807\u3002\n * @param options \u521b\u5efa\u6258\u76d8\u56fe\u6807\u7684\u914d\u7f6e\u9879\u3002\n * @param window_id \u8981\u521b\u5efa\u6258\u76d8\u56fe\u6807\u7684\u7a97\u53e3 ID\uff0c\u9ed8\u8ba4\u4e3a\u5f53\u524d\u6d3b\u52a8\u7a97\u53e3 ID\u3002\n * @returns \u4e00\u4e2a Promise\uff0c\u5728\u521b\u5efa\u6210\u529f\u65f6\u89e3\u6790\u8be5 Promise\uff0c\u6216\u5728\u53d1\u751f\u9519\u8bef\u65f6\u62d2\u7edd\u8be5 Promise\u3002\u6210\u529f\u65f6\u8fd4\u56de\u65b0\u521b\u5efa\u7684\u6258\u76d8\u56fe\u6807 ID\u3002\n */\nexport function create(options: NivaTrayOptions, window_id?: number): Promise<number>;\n")),(0,a.kt)("h2",{id:"nivaapitraydestroy"},"Niva.api.tray.destroy"),(0,a.kt)("pre",null,(0,a.kt)("code",{parentName:"pre",className:"language-ts"},"/**\n * \u9500\u6bc1\u6307\u5b9a\u7684\u6258\u76d8\u56fe\u6807\u3002\n * @param id \u8981\u9500\u6bc1\u7684\u6258\u76d8\u56fe\u6807 ID\u3002\n * @param window_id \u8981\u9500\u6bc1\u6258\u76d8\u56fe\u6807\u7684\u7a97\u53e3 ID\uff0c\u9ed8\u8ba4\u4e3a\u5f53\u524d\u6d3b\u52a8\u7a97\u53e3 ID\u3002\n * @returns \u4e00\u4e2a Promise\uff0c\u5728\u9500\u6bc1\u6210\u529f\u65f6\u89e3\u6790\u8be5 Promise\uff0c\u6216\u5728\u53d1\u751f\u9519\u8bef\u65f6\u62d2\u7edd\u8be5 Promise\u3002\n */\nexport function destroy(id: number, window_id?: number): Promise<void>;\n")),(0,a.kt)("h2",{id:"nivaapitraydestroyall"},"Niva.api.tray.destroyAll"),(0,a.kt)("pre",null,(0,a.kt)("code",{parentName:"pre",className:"language-ts"},"/**\n * \u9500\u6bc1\u6307\u5b9a\u7a97\u53e3\u7684\u6240\u6709\u6258\u76d8\u56fe\u6807\u3002\n * @param window_id \u8981\u9500\u6bc1\u6258\u76d8\u56fe\u6807\u7684\u7a97\u53e3 ID\uff0c\u9ed8\u8ba4\u4e3a\u5f53\u524d\u6d3b\u52a8\u7a97\u53e3 ID\u3002\n * @returns \u4e00\u4e2a Promise\uff0c\u5728\u9500\u6bc1\u6210\u529f\u65f6\u89e3\u6790\u8be5 Promise\uff0c\u6216\u5728\u53d1\u751f\u9519\u8bef\u65f6\u62d2\u7edd\u8be5 Promise\u3002\n */\nexport function destroyAll(window_id?: number): Promise<void>;\n")),(0,a.kt)("h2",{id:"nivaapitraylist"},"Niva.api.tray.list"),(0,a.kt)("pre",null,(0,a.kt)("code",{parentName:"pre",className:"language-ts"},"/**\n * \u83b7\u53d6\u6307\u5b9a\u7a97\u53e3\u5f53\u524d\u5b58\u5728\u7684\u6240\u6709\u6258\u76d8\u56fe\u6807 ID\u3002\n * @param window_id \u8981\u83b7\u53d6\u6258\u76d8\u56fe\u6807 ID \u7684\u7a97\u53e3 ID\uff0c\u9ed8\u8ba4\u4e3a\u5f53\u524d\u6d3b\u52a8\u7a97\u53e3 ID\u3002\n * @returns \u4e00\u4e2a Promise\uff0c\u5728\u83b7\u53d6\u6210\u529f\u65f6\u89e3\u6790\u8be5 Promise\uff0c\u6216\u5728\u53d1\u751f\u9519\u8bef\u65f6\u62d2\u7edd\u8be5 Promise\u3002\u6210\u529f\u65f6\u8fd4\u56de\u6258\u76d8\u56fe\u6807 ID \u7684\u6570\u7ec4\u3002\n */\nexport function list(window_id?: number): Promise<number[]>;\n")),(0,a.kt)("h2",{id:"nivaapitrayupdate"},"Niva.api.tray.update"),(0,a.kt)("pre",null,(0,a.kt)("code",{parentName:"pre",className:"language-ts"},"/**\n * \u66f4\u65b0\u6307\u5b9a\u6258\u76d8\u56fe\u6807\u7684\u914d\u7f6e\u9879\u3002\n * @param id \u8981\u66f4\u65b0\u7684\u6258\u76d8\u56fe\u6807 ID\u3002\n * @param options \u65b0\u7684\u6258\u76d8\u56fe\u6807\u914d\u7f6e\u9879\u3002\n * @param window_id \u8981\u66f4\u65b0\u6258\u76d8\u56fe\u6807\u7684\u7a97\u53e3 ID\uff0c\u9ed8\u8ba4\u4e3a\u5f53\u524d\u6d3b\u52a8\u7a97\u53e3 ID\u3002\n * @returns \u4e00\u4e2a Promise\uff0c\u5728\u66f4\u65b0\u6210\u529f\u65f6\u89e3\u6790\u8be5 Promise\uff0c\u6216\u5728\u53d1\u751f\u9519\u8bef\u65f6\u62d2\u7edd\u8be5 Promise\u3002\n */\nexport function update(id: number, options: NivaTrayUpdateOptions, window_id?: number): Promise<void>;\n")))}d.isMDXComponent=!0}}]);