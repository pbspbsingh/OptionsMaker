import type { CSSProperties } from "react";

export const Connected = (props: any) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    fill="#5fff15"
    stroke="#5fff15"
    viewBox="-0.8 -0.8 17.6 17.6"
    {...props}
  >
    <g id="SVGRepo_iconCarrier">
      <g id="Layer_2" data-name="Layer 2">
        <path
          id="Layer_1-2"
          d="M8 7.8a2 2 0 1 1 2-2 2 2 0 0 1-2 2Zm0-3a1 1 0 1 0 1 1 1 1 0 0 0-1-1Zm5.66 6.66a8 8 0 0 0 0-11.31.5.5 0 0 0-.71 0 .48.48 0 0 0 0 .7 7 7 0 0 1 0 9.9.5.5 0 0 0 0 .71.5.5 0 0 0 .35.15.5.5 0 0 0 .36-.15Zm-2.12-2.12a5 5 0 0 0 0-7.07.5.5 0 0 0-.71 0 .48.48 0 0 0 0 .7 4 4 0 0 1 0 5.66.5.5 0 0 0 0 .71.5.5 0 0 0 .35.15.5.5 0 0 0 .36-.15Zm-6.37 0a.5.5 0 0 0 0-.71 4 4 0 0 1 0-5.63.48.48 0 0 0 0-.7.5.5 0 0 0-.71 0 5 5 0 0 0 0 7.07.5.5 0 0 0 .36.15.5.5 0 0 0 .35-.18Zm-2.12 2.12a.5.5 0 0 0 0-.71 7 7 0 0 1 0-9.9.48.48 0 0 0 0-.7.5.5 0 0 0-.71 0 8 8 0 0 0 0 11.31.5.5 0 0 0 .36.15.5.5 0 0 0 .35-.15Z"
          data-name="Layer 1"
        ></path>
      </g>
    </g>
  </svg>
);

export const NotConnected = (props: any) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    fill="#ff0700"
    stroke="#ff0700"
    viewBox="-0.8 -0.8 17.6 17.6"
    {...props}
  >
    <g>
      <g id="Layer_2" data-name="Layer 2">
        <path
          id="Layer_1-2"
          d="m6.07 7.49 2.44 2.44A2 2 0 0 1 8 10a2 2 0 0 1-2-2 2 2 0 0 1 .07-.51ZM10 8a2 2 0 0 0-2-2 2 2 0 0 0-.51.07l2.44 2.44A2 2 0 0 0 10 8Zm.83 3.53L13 13.66l2.2 2.19a.48.48 0 0 0 .7 0 .48.48 0 0 0 0-.7L14 13.29l.32-.39a8 8 0 0 0-.65-10.56.51.51 0 0 0-.71 0 .5.5 0 0 0 0 .71 7 7 0 0 1 .65 9.14l-.31.39-1.44-1.43a4 4 0 0 0 .32-.39 5 5 0 0 0-.63-6.3.51.51 0 0 0-.71 0 .5.5 0 0 0 0 .71A4 4 0 0 1 12 8a4.06 4.06 0 0 1-.56 2 4 4 0 0 1-.29.42l-6-6-2.1-2.08L.85.15a.48.48 0 0 0-.7 0 .48.48 0 0 0 0 .7L2 2.71l-.32.39a8 8 0 0 0 .65 10.56.54.54 0 0 0 .36.14.52.52 0 0 0 .35-.14.5.5 0 0 0 0-.71 7 7 0 0 1-.64-9.14l.31-.39 1.44 1.43a4 4 0 0 0-.32.39 5 5 0 0 0 .63 6.3.54.54 0 0 0 .36.14.52.52 0 0 0 .35-.14.5.5 0 0 0 0-.71A4 4 0 0 1 4 8a4.06 4.06 0 0 1 .56-2 4 4 0 0 1 .29-.42Z"
          data-name="Layer 1"
        ></path>
      </g>
    </g>
  </svg>
);

export const Bucket = ({ fill = "#4312d6ff", stroke = "#7e696b8e", ...props }: { [x: string]: string }) => (
  <svg
    viewBox="0 -0.5 17 17"
    xmlns="http://www.w3.org/2000/svg"
    xmlnsXlink="http://www.w3.org/1999/xlink"
    className="si-glyph si-glyph-bucket"
    fill={fill}
    stroke={stroke}
    {...props}
  >
    <g id="SVGRepo_bgCarrier" strokeWidth={0} />
    <g
      id="SVGRepo_tracerCarrier"
      strokeLinecap="round"
      strokeLinejoin="round"
    />
    <g id="SVGRepo_iconCarrier">
      <defs />
      <g stroke="none" strokeWidth={1} fill="none" fillRule="evenodd">
        <g transform="translate(1.000000, 0.000000)">
          <path
            d="M7.759,1.143 C7.252,0.634 5.403,1.664 3.629,3.445 C1.857,5.227 0.828,7.082 1.336,7.591 C1.842,8.099 3.69,7.07 5.465,5.287 C7.237,3.506 8.266,1.65 7.759,1.143 L7.759,1.143 Z"
            className=""
          />
          <path
            d="M15.737,7.881 L14.834,6.985 C14.507,7.895 13.859,8.674 12.905,9.235 C11.888,9.835 10.612,10.151 9.214,10.151 C8.423,10.151 7.623,10.048 6.836,9.846 C6.682,9.806 6.554,9.709 6.473,9.573 C6.392,9.437 6.369,9.276 6.41,9.123 C6.478,8.86 6.715,8.677 6.985,8.677 C7.036,8.677 7.087,8.684 7.133,8.697 C7.828,8.877 8.531,8.968 9.224,8.968 C10.399,8.968 11.463,8.707 12.301,8.215 C13.094,7.747 13.615,7.104 13.808,6.354 C13.836,6.24 13.831,6.121 13.846,6.004 L7.862,0.062 C7.862,0.062 6.173,-0.244 3.044,2.899 C-0.089,6.044 0.034,7.834 0.034,7.834 C0.034,7.834 7.194,15.023 7.911,15.741 C8.628,16.461 10.902,15.533 13.25,13.177 C15.598,10.814 16.343,8.489 15.737,7.881 L15.737,7.881 Z M1.336,7.59 C0.828,7.081 1.857,5.227 3.629,3.444 C5.402,1.663 7.252,0.633 7.759,1.142 C8.266,1.65 7.238,3.505 5.465,5.286 C3.69,7.068 1.842,8.098 1.336,7.59 L1.336,7.59 Z"
            fill={fill}
            className="si-glyph-fill"
          />
          <path
            d="M14.864,6.621 C15.554,3.936 13.089,0.974 9.366,0.016 C9.099,-0.049 8.831,0.107 8.762,0.371 C8.695,0.635 8.854,0.904 9.118,0.973 C12.181,1.76 14.275,4.022 13.951,6.109 L14.757,6.908 C14.791,6.811 14.839,6.721 14.864,6.621 L14.864,6.621 Z"
            fill={fill}
            className="si-glyph-fill"
          />
        </g>
      </g>
    </g>
  </svg>
);

export const Loader = () => {
  const styles: { [key: string]: CSSProperties } = {
    container: {
      flex: 1,
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      background: 'none',
    },
    spinner: {
      width: '60px',
      height: '60px',
      border: '6px solid #e0e0e0',
      borderTop: '6px solid #3b82f6',
      borderRadius: '50%',
      animation: 'spin 1s linear infinite',
    },
    text: {
      marginTop: '20px',
      color: '#a79a9aff',
      fontSize: '16px',
      fontWeight: '500',
      animation: 'pulse 1.5s ease-in-out infinite',
    },
  };
  return (
    <div style={styles.container}>
      <div style={styles.spinner}></div>
      <p style={styles.text}>Loading...</p>

      <style>{`
        @keyframes spin {
          0% { transform: rotate(0deg); }
          100% { transform: rotate(360deg); }
        }
        
        @keyframes pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.5; }
        }
      `}</style>
    </div>
  );
};
