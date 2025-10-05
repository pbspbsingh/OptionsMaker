import { useSearchParams } from "react-router";

export type GroupParam = {
    filterName: string,
    groups: Group[],
};

export type Group = {
    name: string,
    count: number,
    selected: boolean,
};

export default function GroupView({ filterName, groups }: GroupParam) {
    const [searchParams, setSearchParams] = useSearchParams();
    const toggleSector = (groupName: string) => {
        const groupSet = new Set(groups.map(g => g.name));
        const exitingGroups = searchParams.getAll(filterName).filter(g => groupSet.has(g));
        const index = exitingGroups.indexOf(groupName);
        if (index === -1) {
            exitingGroups.push(groupName);
        } else {
            exitingGroups.splice(index, 1);
        }
        setSearchParams(prev => {
            const newParams = new URLSearchParams(prev);
            newParams.delete(filterName);
            exitingGroups.forEach(name => {
                newParams.append(filterName, name);
            });
            return newParams;
        });
    };

    return (
        <section className='group'>
            <h5>{filterName} total: {groups
                .filter(g => g.selected)
                .map(g => g.count)
                .reduce((acc, num) => acc + num, 0)}
            </h5>
            {groups.map(group => (
                <button
                    className={group.selected ? 'primary' : 'outline secondary'}
                    key={group.name}
                    onClick={() => toggleSector(group.name)}>
                    {group.name}({group.count})
                </button>
            ))}
        </section>
    );
}
