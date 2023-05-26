import { useEffect } from 'react';
import { useLocale, useModal } from '../models/app.model';

const useCompareRemoteVersion = () => {
    const modal = useModal();
    const locale = useLocale();
    useEffect(() => {
        Niva.api.http.get('https://api.github.com/repos/bramblex/niva/releases/latest')
            .then(res => {
                const remoteVersion = JSON.parse(res?.body)?.tag_name;
                Niva.api.process.version().then(localVersion => {
                    if (remoteVersion && localVersion !== remoteVersion) {
                        modal.alert(locale.t('NEWER_VERSION_TIP'), locale.t('NEWER_VERSION_TEXT', {
                            version: remoteVersion
                        }))
                    }
                })
            })
    }, [])
}

export default useCompareRemoteVersion;