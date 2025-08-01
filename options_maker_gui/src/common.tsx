import { useState } from "react";

export interface TextEditProps {
    initVal: string,
    onUpdate?: (newVal: string) => void;
}

export function TextEdit({ initVal, onUpdate }: TextEditProps) {
    const [isEditing, setIsEditing] = useState(false);
    return (
        <>
            {isEditing ?
                <input
                    type="text"
                    defaultValue={initVal}
                    onChange={e => onUpdate?.call(null, e.target.value)}
                    onBlur={() => setIsEditing(false)}
                /> :
                <span onDoubleClick={() => setIsEditing(true)}>{initVal}</span>}
        </>
    );
}
